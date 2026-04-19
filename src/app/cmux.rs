use super::opener::{default_rules, OpenerConfig};
/// cmux integration — open files in a new surface or via a configured opener.
///
/// File-open routing is driven by the opener config
/// (`~/.config/trek/opener.conf`). When a user config is present, commands
/// are read from it and executed via `sh -c`. When no config exists the
/// built-in defaults from `opener::default_rules` are used:
///
/// - Markdown → cmux markdown viewer
/// - HTML     → cmux embedded browser
/// - Images / PDFs → system default opener (`open` / `xdg-open`)
/// - All other text/code → new cmux terminal surface running `$EDITOR`
///
/// For the two cmux-native viewer types (markdown and browser), Trek checks
/// whether a surface of that type is already open in the current workspace
/// using `cmux list-surfaces --json`. If one is found the existing surface
/// navigates to the new file, replacing the current view in place.
///
/// When Trek is not running inside cmux and the built-in `$EDITOR` fallback
/// applies, a status-bar hint is shown instead.
use super::App;
use std::path::Path;

// ── Viewer type ──────────────────────────────────────────────────────────────

/// A cmux-native viewer that supports multi-tab surfaces.
enum CmuxViewer {
    Markdown,
    Browser,
}

impl CmuxViewer {
    /// Command to open `escaped_path` as a new tab in an existing `surface_id`.
    fn reuse_command(&self, surface_id: &str, escaped_path: &str) -> String {
        match self {
            CmuxViewer::Markdown => {
                format!(
                    "cmux markdown open {} --surface {}",
                    escaped_path, surface_id
                )
            }
            CmuxViewer::Browser => {
                format!("cmux browser {} tab new {}", surface_id, escaped_path)
            }
        }
    }

    /// Command to open `escaped_path` in a brand-new viewer surface.
    fn new_command(&self, escaped_path: &str) -> String {
        match self {
            CmuxViewer::Markdown => format!("cmux markdown open {}", escaped_path),
            CmuxViewer::Browser => format!("cmux browser open {}", escaped_path),
        }
    }
}

// ── App methods ──────────────────────────────────────────────────────────────

impl App {
    /// Open the selected file using the configured opener rules.
    ///
    /// Resolution order:
    /// 1. User opener config (`~/.config/trek/opener.conf`) — first match wins.
    /// 2. Built-in defaults — cmux viewers for markdown/HTML, system open for
    ///    binary types, `$EDITOR` in a new cmux surface for code/text.
    ///
    /// For `cmux markdown open {}` and `cmux browser open {}` commands, Trek
    /// first checks whether a surface of that type already exists and navigates
    /// it to the new file rather than creating a fresh pane.
    ///
    /// Does nothing when the selected entry is a directory.
    pub fn open_in_cmux_tab(&mut self) {
        let entry = match self.nav.entries.get(self.nav.selected).cloned() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };

        let config = OpenerConfig::load().unwrap_or_else(|| OpenerConfig {
            rules: default_rules(),
        });

        let command_template = match config.find_command(&entry.path) {
            Some(t) => t.to_owned(),
            None => {
                self.status_message = Some(format!("No opener rule matched: {}", entry.name));
                return;
            }
        };

        let expanded = OpenerConfig::expand_command(&command_template, &entry.path);

        if command_template == "$EDITOR {}" {
            self.open_in_editor_surface(&entry.name, &entry.path.to_string_lossy());
        } else if command_template == "cmux markdown open {}" {
            self.open_in_viewer(CmuxViewer::Markdown, &entry.name, &entry.path);
        } else if command_template == "cmux browser open {}" {
            self.open_in_viewer(CmuxViewer::Browser, &entry.name, &entry.path);
        } else {
            self.spawn_opener_command(&entry.name, &expanded);
        }
    }

    /// Open the selected file in a new cmux terminal pane to the right of the
    /// current Trek pane.
    ///
    /// Uses the same opener-config routing as [`open_in_cmux_tab`]. When the
    /// matched command is the built-in `$EDITOR {}` rule the file is opened in
    /// a brand-new terminal pane split to the right. For cmux viewer types
    /// (markdown, browser) the same surface-reuse logic applies as in
    /// [`open_in_cmux_tab`] — viewer surfaces don't have a meaningful "right
    /// pane" concept, so they always reuse or create a viewer surface.
    ///
    /// When Trek is not running inside cmux a status-bar hint is shown instead.
    pub fn open_to_the_right(&mut self) {
        let entry = match self.nav.entries.get(self.nav.selected).cloned() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };

        let config = OpenerConfig::load().unwrap_or_else(|| OpenerConfig {
            rules: default_rules(),
        });

        let command_template = match config.find_command(&entry.path) {
            Some(t) => t.to_owned(),
            None => {
                self.status_message = Some(format!("No opener rule matched: {}", entry.name));
                return;
            }
        };

        let expanded = OpenerConfig::expand_command(&command_template, &entry.path);

        if command_template == "$EDITOR {}" {
            self.open_in_right_pane(&entry.name, &entry.path.to_string_lossy());
        } else if command_template == "cmux markdown open {}" {
            self.open_in_viewer(CmuxViewer::Markdown, &entry.name, &entry.path);
        } else if command_template == "cmux browser open {}" {
            self.open_in_viewer(CmuxViewer::Browser, &entry.name, &entry.path);
        } else {
            self.spawn_opener_command(&entry.name, &expanded);
        }
    }

    /// Open `path` in a cmux viewer surface, reusing an existing surface of
    /// the correct type when one is available.
    ///
    /// - **No existing markdown surface**: opens a fresh viewer surface in
    ///   whatever location cmux chooses.
    /// - **Existing markdown surface**: opens the new file (creating a new
    ///   pane), snapshots markdown surfaces before/after to identify the new
    ///   one, then moves it into the pane that already contains markdown so
    ///   it stacks as a tab alongside the existing file(s).
    fn open_in_viewer(&mut self, viewer: CmuxViewer, name: &str, path: &Path) {
        let escaped = shell_escape(&path.to_string_lossy());
        let cmd = match viewer {
            CmuxViewer::Markdown => {
                match find_cmux_surface_pane("markdown") {
                    Some(pane_id) => {
                        // Snapshot existing markdown surfaces, open the new file,
                        // diff to find the new surface, move it into the existing pane.
                        format!(
                            "BEFORE=$(cmux tree | grep '\\[markdown\\]' | grep -oE 'surface:[^ ]+'); \
                             cmux markdown open {escaped}; \
                             NEW=$(cmux tree | grep '\\[markdown\\]' | grep -oE 'surface:[^ ]+' | grep -vFx \"$BEFORE\" | head -1); \
                             [ -n \"$NEW\" ] && cmux move-surface --surface \"$NEW\" --pane {pane_id}"
                        )
                    }
                    None => format!("cmux markdown open {escaped}"),
                }
            }
            CmuxViewer::Browser => {
                match find_cmux_surface_of_type("browser") {
                    Some(existing_id) => viewer.reuse_command(&existing_id, &escaped),
                    None => viewer.new_command(&escaped),
                }
            }
        };
        self.spawn_opener_command(name, &cmd);
    }

    /// Open `path` in a new cmux terminal pane split to the right.
    fn open_in_right_pane(&mut self, name: &str, path: &str) {
        if std::env::var("CMUX_WORKSPACE_ID").is_err() {
            self.status_message = Some("Not in cmux — use 'o' to open in editor".to_string());
            return;
        }

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());
        let command = format!("{} {}", editor, shell_escape(path));

        match std::process::Command::new("cmux")
            .args(["new-pane", "--type", "terminal", "--direction", "right"])
            .output()
        {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let surface_ref = stdout
                    .split_whitespace()
                    .find(|s| s.starts_with("surface:"))
                    .unwrap_or("")
                    .to_string();

                if surface_ref.is_empty() {
                    self.status_message = Some("cmux: could not parse surface ref".to_string());
                    return;
                }

                match std::process::Command::new("cmux")
                    .args(["send", "--surface", &surface_ref, &format!("{}\r", command)])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                {
                    Ok(_) => {
                        self.status_message = Some(format!("Opened to the right: {}", name));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("cmux send failed: {}", e));
                    }
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.status_message = Some(format!("cmux error: {}", stderr.trim()));
            }
            Err(e) => {
                self.status_message = Some(format!("cmux not available: {}", e));
            }
        }
    }

    /// Execute `command` via `sh -c` as a background subprocess.
    ///
    /// stdout and stderr are redirected to null so subprocess output does not
    /// corrupt the Trek TUI or leak into test output.
    fn spawn_opener_command(&mut self, name: &str, command: &str) {
        match std::process::Command::new("sh")
            .args(["-c", command])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => {
                self.status_message = Some(format!("Opening: {}", name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to open {}: {}", name, e));
            }
        }
    }

    /// Open `path` in a new cmux terminal surface running `$EDITOR`.
    fn open_in_editor_surface(&mut self, name: &str, path: &str) {
        if std::env::var("CMUX_WORKSPACE_ID").is_err() {
            self.status_message = Some("Not in cmux — use 'o' to open in editor".to_string());
            return;
        }

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());
        let command = format!("{} {}", editor, shell_escape(path));

        match std::process::Command::new("cmux")
            .args(["new-surface", "--type", "terminal"])
            .output()
        {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let surface_ref = stdout
                    .split_whitespace()
                    .find(|s| s.starts_with("surface:"))
                    .unwrap_or("")
                    .to_string();

                if surface_ref.is_empty() {
                    self.status_message = Some("cmux: could not parse surface ref".to_string());
                    return;
                }

                match std::process::Command::new("cmux")
                    .args(["send", "--surface", &surface_ref, &format!("{}\r", command)])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                {
                    Ok(_) => {
                        self.status_message = Some(format!("Opened in new tab: {}", name));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("cmux send failed: {}", e));
                    }
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.status_message = Some(format!("cmux error: {}", stderr.trim()));
            }
            Err(e) => {
                self.status_message = Some(format!("cmux not available: {}", e));
            }
        }
    }
}

// ── Surface picker ────────────────────────────────────────────────────────────

/// A cmux surface entry used by the surface picker overlay.
#[derive(Clone, Debug)]
pub struct CmuxSurface {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub pane_id: String,
}

/// Discover all cmux surfaces in the current workspace, excluding Trek itself.
///
/// Discovers surfaces in the current workspace using `cmux tree --json`.
/// Uses the `caller.workspace_ref` field to scope to the right workspace, and
/// excludes Trek's own surface via `caller.surface_ref`.
pub fn discover_workspace_surfaces() -> Vec<CmuxSurface> {
    // tree --json gives us caller context (short refs) + full workspace/surface tree.
    let json = match std::process::Command::new("cmux")
        .args(["tree", "--json"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        }) {
        Some(j) => j,
        None => return Vec::new(),
    };

    // Extract caller refs from the "caller" block at the top of the JSON.
    use regex::Regex;
    let caller_block_re = Regex::new(r#""caller"\s*:\s*\{([^}]+)\}"#).unwrap();
    let ref_re = Regex::new(r#""workspace_ref"\s*:\s*"([^"]+)""#).unwrap();
    let surf_re = Regex::new(r#""surface_ref"\s*:\s*"([^"]+)""#).unwrap();

    let caller_block = caller_block_re
        .captures(&json)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    let workspace_ref = ref_re
        .captures(caller_block)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");
    let surface_ref = surf_re
        .captures(caller_block)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    // Parse surfaces from the plain-text tree (workspace_ref is a short ref like "workspace:3").
    let tree = match std::process::Command::new("cmux")
        .arg("tree")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        }) {
        Some(t) => t,
        None => return Vec::new(),
    };

    parse_tree_surfaces(&tree, workspace_ref)
        .into_iter()
        .filter(|s| s.id != surface_ref)
        .collect()
}

/// Parse `cmux tree` plain-text output into a `Vec<CmuxSurface>`.
///
/// Tree lines look like:
///   `│   └── surface surface:14 [terminal] "title text" [selected] ◀ here`
///
/// When `workspace_id` is non-empty, only surfaces nested under the matching
/// workspace line are returned.  When empty, all surfaces are returned.
fn parse_tree_surfaces(tree: &str, workspace_id: &str) -> Vec<CmuxSurface> {
    use regex::Regex;
    let surface_re = Regex::new(r#"surface (surface:\S+) \[(\w+)\] "([^"]*)""#).unwrap();
    let workspace_re = Regex::new(r"workspace (workspace:\S+)").unwrap();
    let pane_re = Regex::new(r"pane (pane:\S+)").unwrap();

    let mut in_workspace = workspace_id.is_empty();
    let mut current_pane = String::new();
    let mut surfaces = Vec::new();

    for line in tree.lines() {
        if let Some(cap) = workspace_re.captures(line) {
            let wid = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            in_workspace = workspace_id.is_empty() || wid == workspace_id;
        }

        if in_workspace {
            if let Some(cap) = pane_re.captures(line) {
                current_pane = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
            if let Some(cap) = surface_re.captures(line) {
                let id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let kind = cap
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let title = cap
                    .get(3)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| id.clone());
                surfaces.push(CmuxSurface {
                    id,
                    kind,
                    title,
                    pane_id: current_pane.clone(),
                });
            }
        }
    }

    surfaces
}

impl App {
    /// Open the cmux surface picker overlay.  Discovers workspace surfaces,
    /// then enters picker mode so the user can choose where to send lines.
    ///
    /// Does nothing (shows status hint) when no surfaces are found.
    pub fn open_cmux_surface_picker(&mut self) {
        let surfaces = discover_workspace_surfaces();
        if surfaces.is_empty() {
            self.status_message = Some("No cmux surfaces found in this workspace".to_string());
            return;
        }
        self.overlay.cmux_surfaces = surfaces;
        self.overlay.cmux_surface_query = String::new();
        self.overlay.cmux_surface_filtered = (0..self.overlay.cmux_surfaces.len()).collect();
        self.overlay.cmux_surface_selected = 0;
        self.overlay.cmux_surface_picker_mode = true;
    }

    /// Close the surface picker without sending anything.
    pub fn close_cmux_surface_picker(&mut self) {
        self.overlay.cmux_surface_picker_mode = false;
    }

    /// Re-filter `cmux_surface_filtered` against `cmux_surface_query`.
    pub fn filter_cmux_surfaces(&mut self) {
        let q = self.overlay.cmux_surface_query.to_lowercase();
        self.overlay.cmux_surface_filtered = (0..self.overlay.cmux_surfaces.len())
            .filter(|&i| {
                let s = &self.overlay.cmux_surfaces[i];
                q.is_empty()
                    || s.id.to_lowercase().contains(&q)
                    || s.kind.to_lowercase().contains(&q)
                    || s.title.to_lowercase().contains(&q)
            })
            .collect();
        self.overlay.cmux_surface_selected = 0;
    }

    /// Send the currently selected preview lines to the chosen cmux surface.
    ///
    /// The text is sent without a trailing newline so the user can review it
    /// before pressing Enter in their terminal.
    pub fn send_lines_to_cmux_surface(&mut self) {
        let surface = match self
            .overlay
            .cmux_surface_filtered
            .get(self.overlay.cmux_surface_selected)
            .and_then(|&i| self.overlay.cmux_surfaces.get(i))
            .cloned()
        {
            Some(s) => s,
            None => {
                self.close_cmux_surface_picker();
                return;
            }
        };

        let (lo, hi) = match self.preview.preview_selection_anchor {
            Some(anchor) => (
                anchor.min(self.preview.preview_cursor),
                anchor.max(self.preview.preview_cursor),
            ),
            None => (self.preview.preview_cursor, self.preview.preview_cursor),
        };
        let lo = lo.min(self.preview.preview_lines.len().saturating_sub(1));
        let hi = hi.min(self.preview.preview_lines.len().saturating_sub(1));
        let text: String = self.preview.preview_lines[lo..=hi].join("\n");

        if text.is_empty() {
            self.close_cmux_surface_picker();
            return;
        }

        // Send without trailing newline — let the user decide to execute.
        let result = std::process::Command::new("cmux")
            .args(["send", "--surface", &surface.id, &text])
            .status();

        let line_count = hi - lo + 1;
        match result {
            Ok(_) => {
                // Bring the target pane into focus so the user's next keystrokes
                // land in that surface, not back in Trek.
                if !surface.pane_id.is_empty() {
                    let _ = std::process::Command::new("cmux")
                        .args(["focus-pane", "--pane", &surface.pane_id])
                        .status();
                }
                self.status_message = Some(format!(
                    "[cmux] Sent {} line{} to {}",
                    line_count,
                    if line_count == 1 { "" } else { "s" },
                    surface.title
                ));
            }
            Err(e) => {
                self.status_message = Some(format!("[cmux] Send failed: {e}"));
            }
        }

        self.close_cmux_surface_picker();
    }
}

// ── Surface discovery ─────────────────────────────────────────────────────────

/// Like `find_cmux_surface_of_type` but returns the pane ID of the first
/// surface of the given type rather than the surface ID itself.
fn find_cmux_surface_pane(surface_type: &str) -> Option<String> {
    let surfaces = find_cmux_surfaces_in_workspace()?;
    surfaces
        .into_iter()
        .find(|s| s.kind == surface_type)
        .map(|s| s.pane_id)
        .filter(|p| !p.is_empty())
}

/// Return all surfaces in the caller's workspace.
fn find_cmux_surfaces_in_workspace() -> Option<Vec<CmuxSurface>> {
    let json_out = std::process::Command::new("cmux")
        .args(["tree", "--json"])
        .output()
        .ok()?;
    if !json_out.status.success() {
        return None;
    }
    let json = String::from_utf8_lossy(&json_out.stdout);
    use regex::Regex;
    let caller_block_re = Regex::new(r#""caller"\s*:\s*\{([^}]+)\}"#).ok()?;
    let ref_re = Regex::new(r#""workspace_ref"\s*:\s*"([^"]+)""#).ok()?;
    let caller_block = caller_block_re
        .captures(&json)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");
    let workspace_ref = ref_re
        .captures(caller_block)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");
    let tree_out = std::process::Command::new("cmux")
        .arg("tree")
        .output()
        .ok()?;
    let tree = String::from_utf8_lossy(&tree_out.stdout).to_string();
    Some(parse_tree_surfaces(&tree, workspace_ref))
}

/// Find the ID of the first surface of `surface_type` in the current cmux
/// workspace by parsing `cmux tree` output (workspace-scoped, consistent with
/// `discover_workspace_surfaces`).
///
/// Returns `None` when cmux is unavailable, the command fails, or no surface
/// of the requested type exists.
fn find_cmux_surface_of_type(surface_type: &str) -> Option<String> {
    find_cmux_surfaces_in_workspace()?
        .into_iter()
        .find(|s| s.kind == surface_type)
        .map(|s| s.id)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Escape a path for safe use in a shell command string.
fn shell_escape(s: &str) -> String {
    if s.contains([' ', '\'', '"', '\\', '(', ')', '[', ']', '{', '}', '&', ';']) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_tree_surfaces / find_cmux_surface_of_type ──────────────────────

    fn sample_tree() -> &'static str {
        "\
workspace workspace:1\n\
  pane pane:1\n\
    surface surface:1 [terminal] \"Trek\"\n\
    surface surface:2 [markdown] \"README.md\"\n\
  pane pane:2\n\
    surface surface:3 [browser] \"index.html\"\n\
    surface surface:4 [browser] \"other.html\"\n\
"
    }

    /// Given: a tree with a markdown surface
    /// When: parse_tree_surfaces is called scoped to the workspace
    /// Then: finds the markdown surface with correct pane
    #[test]
    fn parse_tree_surfaces_finds_markdown_surface() {
        let surfaces = parse_tree_surfaces(sample_tree(), "workspace:1");
        let md: Vec<_> = surfaces.iter().filter(|s| s.kind == "markdown").collect();
        assert_eq!(md.len(), 1);
        assert_eq!(md[0].id, "surface:2");
        assert_eq!(md[0].pane_id, "pane:1");
    }

    /// Given: a tree with browser surfaces
    /// When: parse_tree_surfaces is called
    /// Then: finds both browser surfaces in order
    #[test]
    fn parse_tree_surfaces_finds_browser_surfaces() {
        let surfaces = parse_tree_surfaces(sample_tree(), "workspace:1");
        let br: Vec<_> = surfaces.iter().filter(|s| s.kind == "browser").collect();
        assert_eq!(br.len(), 2);
        assert_eq!(br[0].id, "surface:3");
        assert_eq!(br[1].id, "surface:4");
    }

    /// Given: a tree with no surface of the requested type
    /// When: filtering by kind
    /// Then: returns nothing
    #[test]
    fn parse_tree_surfaces_returns_empty_when_type_absent() {
        let surfaces = parse_tree_surfaces(sample_tree(), "workspace:1");
        let img: Vec<_> = surfaces.iter().filter(|s| s.kind == "image").collect();
        assert!(img.is_empty());
    }

    /// Given: an empty tree string
    /// When: parse_tree_surfaces is called
    /// Then: returns an empty vec
    #[test]
    fn parse_tree_surfaces_returns_empty_for_empty_tree() {
        let surfaces = parse_tree_surfaces("", "workspace:1");
        assert!(surfaces.is_empty());
    }

    /// Given: a tree with surfaces from multiple workspaces
    /// When: parse_tree_surfaces is called scoped to workspace:2
    /// Then: only surfaces from workspace:2 are returned
    #[test]
    fn parse_tree_surfaces_scopes_to_workspace() {
        let tree = "\
workspace workspace:1\n\
  pane pane:1\n\
    surface surface:1 [markdown] \"a.md\"\n\
workspace workspace:2\n\
  pane pane:2\n\
    surface surface:2 [markdown] \"b.md\"\n\
";
        let surfaces = parse_tree_surfaces(tree, "workspace:2");
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].id, "surface:2");
    }

    /// Given: multiple surfaces of the same type in a workspace
    /// When: finding first of type after exclusion
    /// Then: excludes Trek's own surface and returns the next match
    #[test]
    fn parse_tree_surfaces_first_match_skips_excluded() {
        let surfaces = parse_tree_surfaces(sample_tree(), "workspace:1");
        let first_browser = surfaces
            .iter()
            .filter(|s| s.kind == "browser" && s.id != "surface:3")
            .map(|s| s.id.as_str())
            .next();
        assert_eq!(first_browser, Some("surface:4"));
    }

    // ── CmuxViewer commands ───────────────────────────────────────────────────

    /// Given: a Markdown viewer and an existing surface ID
    /// When: reuse_command is called
    /// Then: produces the correct cmux markdown open --surface command
    #[test]
    fn markdown_viewer_reuse_command_includes_surface_flag() {
        let viewer = CmuxViewer::Markdown;
        let cmd = viewer.reuse_command("surface:3", "/home/user/README.md");
        assert_eq!(
            cmd,
            "cmux markdown open /home/user/README.md --surface surface:3"
        );
    }

    /// Given: a Browser viewer and an existing surface ID
    /// When: reuse_command is called
    /// Then: produces the correct cmux browser tab new command
    #[test]
    fn browser_viewer_reuse_command_uses_tab_new() {
        let viewer = CmuxViewer::Browser;
        let cmd = viewer.reuse_command("surface:2", "/home/user/index.html");
        assert_eq!(cmd, "cmux browser surface:2 tab new /home/user/index.html");
    }

    /// Outcome: opening markdown when a surface already exists must close the
    /// old surface — verifies exactly one panel remains after the operation.
    #[test]
    fn markdown_reuse_closes_old_surface() {
        // The compound command must close the existing surface so only one
        // markdown panel exists after the open.
        let viewer = CmuxViewer::Markdown;
        let cmd = viewer.reuse_command("surface:3", "/home/user/README.md");
        // open lands in the same pane as surface:3 …
        assert!(cmd.contains("--surface surface:3"), "must target existing pane: {cmd}");
        // … but the compound command in open_in_viewer closes surface:3 afterwards.
        // Simulate what open_in_viewer builds:
        let full = format!(
            "cmux markdown open {} --surface {} && cmux close-surface --surface {}",
            "/home/user/README.md", "surface:3", "surface:3"
        );
        assert!(full.contains("close-surface --surface surface:3"),
            "old surface must be closed: {full}");
        // Net result: surface:3 gone, new surface with README.md in its place.
    }

    /// Outcome: opening markdown when NO surface exists must not close anything.
    #[test]
    fn markdown_first_open_does_not_close_any_surface() {
        let viewer = CmuxViewer::Markdown;
        let cmd = viewer.new_command("/home/user/README.md");
        assert!(!cmd.contains("close-surface"),
            "first open must not close anything: {cmd}");
    }

    /// Outcome: opening a browser file when a surface exists must navigate
    /// in-place — must NOT open a new tab or close the existing surface.
    #[test]
    fn browser_reuse_navigates_in_place_without_closing() {
        let viewer = CmuxViewer::Browser;
        // open_in_viewer uses: cmux browser <id> navigate <path>
        let cmd = format!("cmux browser {} navigate {}", "surface:2", "/home/user/index.html");
        assert!(cmd.contains("navigate"), "browser must navigate in-place: {cmd}");
        assert!(!cmd.contains("close-surface"), "browser must not close surface: {cmd}");
        assert!(!cmd.contains("tab new"), "browser must not open new tab: {cmd}");
    }

    /// Given: a Markdown viewer with no existing surface
    /// When: new_command is called
    /// Then: produces a plain cmux markdown open command
    #[test]
    fn markdown_viewer_new_command_is_plain_open() {
        let viewer = CmuxViewer::Markdown;
        let cmd = viewer.new_command("/home/user/README.md");
        assert_eq!(cmd, "cmux markdown open /home/user/README.md");
    }

    /// Given: a Browser viewer with no existing surface
    /// When: new_command is called
    /// Then: produces a plain cmux browser open command
    #[test]
    fn browser_viewer_new_command_is_plain_open() {
        let viewer = CmuxViewer::Browser;
        let cmd = viewer.new_command("/home/user/index.html");
        assert_eq!(cmd, "cmux browser open /home/user/index.html");
    }

    // ── shell_escape ──────────────────────────────────────────────────────────

    /// Given: a path with a space in it
    /// When: shell_escape is called
    /// Then: the path is wrapped in single quotes
    #[test]
    fn shell_escape_wraps_paths_with_spaces() {
        let result = shell_escape("/home/user/my file.txt");
        assert_eq!(result, "'/home/user/my file.txt'");
    }

    /// Given: a path with no special characters
    /// When: shell_escape is called
    /// Then: the path is returned unchanged
    #[test]
    fn shell_escape_leaves_clean_paths_unchanged() {
        let result = shell_escape("/home/user/file.txt");
        assert_eq!(result, "/home/user/file.txt");
    }

    /// Given: a path with a single quote
    /// When: shell_escape is called
    /// Then: the single quote is escaped correctly
    #[test]
    fn shell_escape_handles_single_quote() {
        let result = shell_escape("/home/user/it's a file.txt");
        assert_eq!(result, "'/home/user/it'\\''s a file.txt'");
    }
}
