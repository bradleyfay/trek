use super::App;
use crate::icons::icon_for_entry;
use std::path::Path;
use std::process::Command;

impl App {
    pub fn load_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview_lines.clear();
        self.preview_is_diff = false;

        // Hex dump view — first priority.
        if self.hex_view_mode {
            if let Some(entry) = self.entries.get(self.selected).cloned() {
                if !entry.is_dir {
                    self.preview_lines = Self::load_hex_lines(&entry.path);
                }
            }
            return;
        }

        // File compare — third priority, requires exactly 2 files selected.
        if self.file_compare_mode {
            let paths: Vec<_> = self
                .rename_selected
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .filter(|e| !e.is_dir)
                .map(|e| e.path.clone())
                .collect();
            if paths.len() == 2 {
                self.preview_lines = Self::load_file_diff(&paths[0], &paths[1]);
                self.preview_is_diff = true;
            }
            return;
        }

        // Metadata card takes priority over content/diff views.
        if self.meta_preview_mode {
            if let Some(entry) = self.entries.get(self.selected).cloned() {
                self.preview_lines = Self::load_meta_lines(&entry.path);
            }
            return;
        }

        // Git log — third priority.
        if self.git_log_mode {
            if let Some(entry) = self.entries.get(self.selected).cloned() {
                self.preview_lines = Self::load_git_log(&entry.path);
            }
            return;
        }

        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
                // Disk usage breakdown replaces flat listing when active.
                if self.du_preview_mode {
                    self.preview_lines = Self::load_du_lines(&entry.path);
                    return;
                }
                // Directories never show a diff preview.
                if let Ok((children, _)) = Self::read_entries(
                    &entry.path,
                    self.show_hidden,
                    self.sort_mode,
                    self.sort_order,
                ) {
                    self.preview_lines = children
                        .iter()
                        .map(|c| {
                            let icon = icon_for_entry(&c.name, c.is_dir);
                            format!("{} {}", icon, c.name)
                        })
                        .collect();
                }
            } else if self.diff_preview_mode {
                // Show git diff if the file has changes; fall back to raw preview.
                let has_git_change = self
                    .git_status
                    .as_ref()
                    .and_then(|g| g.for_path(&entry.path))
                    .is_some();
                if has_git_change {
                    let diff = Self::load_git_diff(&entry.path);
                    if !diff.is_empty() {
                        self.preview_lines = diff;
                        self.preview_is_diff = true;
                        return;
                    }
                }
                // No diff available — fall back to raw content.
                self.preview_lines = Self::read_file_preview(&entry.path);
            } else {
                self.preview_lines = Self::read_file_preview(&entry.path);
            }
        }
    }

    /// Load `git diff` (unstaged, then staged) for `path` as a list of lines.
    fn load_git_diff(path: &Path) -> Vec<String> {
        let parent = match path.parent() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let path_str = path.to_string_lossy();

        // Try unstaged diff first.
        if let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["diff", "--no-color", "--", path_str.as_ref()])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .take(2000)
                    .map(|l| l.to_string())
                    .collect();
            }
        }

        // Fall back to staged diff.
        if let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["diff", "--cached", "--no-color", "--", path_str.as_ref()])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .take(2000)
                    .map(|l| l.to_string())
                    .collect();
            }
        }

        Vec::new()
    }

    /// Scroll the preview pane up by `lines` lines.
    ///
    /// Clamps at 0 — no-op and no panic when already at the top.
    pub fn scroll_preview_up(&mut self, lines: usize) {
        self.preview_scroll = self.preview_scroll.saturating_sub(lines);
    }

    /// Scroll the preview pane down by `lines` lines.
    ///
    /// Clamps at `preview_lines.len() - 1` — no-op when at the bottom or
    /// when the preview is empty.
    pub fn scroll_preview_down(&mut self, lines: usize) {
        let max_scroll = self.preview_lines.len().saturating_sub(1);
        self.preview_scroll = (self.preview_scroll + lines).min(max_scroll);
    }

    /// Toggle diff preview mode for the currently selected file.
    ///
    /// Has no effect outside a git repo or when the selected item is a
    /// directory or a clean (unmodified) file.
    pub fn toggle_diff_preview(&mut self) {
        let has_git_change = self
            .entries
            .get(self.selected)
            .filter(|e| !e.is_dir)
            .and_then(|e| self.git_status.as_ref().and_then(|g| g.for_path(&e.path)))
            .is_some();

        if has_git_change {
            self.diff_preview_mode = !self.diff_preview_mode;
            if self.diff_preview_mode {
                self.meta_preview_mode = false;
                self.git_log_mode = false; // mutually exclusive
                self.file_compare_mode = false; // mutually exclusive
                self.hex_view_mode = false; // mutually exclusive
                self.du_preview_mode = false; // mutually exclusive
            }
            self.load_preview();
        } else {
            self.status_message = Some("No git changes for this file".to_string());
        }
    }

    /// Toggle two-file compare preview mode.
    ///
    /// Requires exactly 2 non-directory entries in `rename_selected`.
    /// Mutually exclusive with all other special preview modes.
    pub fn toggle_file_compare(&mut self) {
        let any_dir = self
            .rename_selected
            .iter()
            .any(|&i| self.entries.get(i).map(|e| e.is_dir).unwrap_or(false));
        if any_dir {
            self.status_message = Some("File comparison not available for directories".to_string());
            return;
        }
        if self.rename_selected.len() != 2 {
            self.status_message = Some("Select exactly 2 files to compare".to_string());
            return;
        }
        self.file_compare_mode = !self.file_compare_mode;
        if self.file_compare_mode {
            self.diff_preview_mode = false;
            self.meta_preview_mode = false;
            self.git_log_mode = false;
            self.hex_view_mode = false;
            self.du_preview_mode = false;
        }
        self.load_preview();
    }

    /// Toggle hex dump view mode for the currently selected file.
    ///
    /// No-op for directories — shows a status message instead.
    /// Mutually exclusive with all other special preview modes.
    pub fn toggle_hex_view(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.status_message = Some("Hex view not available for directories".to_string());
                return;
            }
        }
        self.hex_view_mode = !self.hex_view_mode;
        if self.hex_view_mode {
            self.meta_preview_mode = false;
            self.diff_preview_mode = false;
            self.git_log_mode = false;
            self.file_compare_mode = false;
            self.du_preview_mode = false;
        }
        self.load_preview();
    }

    /// Produce a hex dump of `path` using `xxd` or `hexdump -C`.
    ///
    /// Caps output at 4 MB to avoid blocking the UI.
    /// Falls back gracefully when neither tool is available.
    pub fn load_hex_lines(path: &Path) -> Vec<String> {
        const MAX_HEX_SIZE: u64 = 4 * 1024 * 1024;

        match std::fs::metadata(path) {
            Ok(meta) if meta.len() > MAX_HEX_SIZE => {
                return vec![
                    String::new(),
                    format!(
                        "  File too large for hex view ({} — limit 4 MB)",
                        super::meta_human_size(meta.len())
                    ),
                ];
            }
            Err(e) => return vec![String::new(), format!("  Error reading file: {}", e)],
            _ => {}
        }

        let tool_available = |bin: &str| -> bool {
            Command::new("which")
                .arg(bin)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        let (cmd, args): (&str, &[&str]) = if tool_available("xxd") {
            ("xxd", &[])
        } else if tool_available("hexdump") {
            ("hexdump", &["-C"])
        } else {
            return vec![
                String::new(),
                "  Hex view requires xxd or hexdump".to_string(),
                String::new(),
                "  Install: brew install vim   (macOS, provides xxd)".to_string(),
                "           apt install xxd    (Debian/Ubuntu)".to_string(),
            ];
        };

        match Command::new(cmd).args(args).arg(path).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .take(2000)
                .map(|l| format!("  {}", l))
                .collect(),
            Ok(out) => vec![
                String::new(),
                format!(
                    "  {} failed: {}",
                    cmd,
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            ],
            Err(e) => vec![String::new(), format!("  Failed to run {}: {}", cmd, e)],
        }
    }

    /// Produce a unified diff between `a` and `b` as preview lines.
    ///
    /// Uses `diff -u` (POSIX); falls back to an informational message on error.
    fn load_file_diff(a: &Path, b: &Path) -> Vec<String> {
        match Command::new("diff").arg("-u").arg(a).arg(b).output() {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.trim().is_empty() {
                    vec![String::new(), "  (files are identical)".to_string()]
                } else {
                    text.lines().take(2000).map(|l| l.to_string()).collect()
                }
            }
            Err(e) => vec![String::new(), format!("  diff failed: {}", e)],
        }
    }

    /// Re-run `git status` for the current directory and refresh the preview.
    pub fn refresh_git_status(&mut self) {
        self.git_status = crate::git::GitStatus::load(&self.cwd);
        self.load_preview();
        self.status_message = Some("Git status refreshed".to_string());
    }

    /// Toggle git log preview mode for the currently selected entry.
    ///
    /// Works for both files and directories. Mutually exclusive with
    /// diff_preview_mode, meta_preview_mode, and hash_preview_mode.
    pub fn toggle_git_log_preview(&mut self) {
        self.git_log_mode = !self.git_log_mode;
        if self.git_log_mode {
            self.diff_preview_mode = false;
            self.meta_preview_mode = false;
            self.file_compare_mode = false;
            self.hex_view_mode = false;
            self.du_preview_mode = false;
        }
        self.load_preview();
    }

    /// Load `git log --oneline -30 -- <path>` output as preview lines.
    ///
    /// Works for both files and directories. Returns an explanatory message
    /// on failure or when there are no commits for the path.
    fn load_git_log(path: &Path) -> Vec<String> {
        let parent = match path.parent() {
            Some(p) if p.as_os_str().is_empty() => Path::new("."),
            Some(p) => p,
            None => return vec!["  (unable to determine parent directory)".to_string()],
        };
        let path_str = path.to_string_lossy();
        match Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["log", "--oneline", "-30", "--", path_str.as_ref()])
            .output()
        {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.trim().is_empty() {
                    vec!["  (no commits for this path yet)".to_string()]
                } else {
                    text.lines().map(|l| format!("  {}", l)).collect()
                }
            }
            _ => vec!["  (git log failed — not a git repository?)".to_string()],
        }
    }

    /// Toggle disk usage preview mode for the selected directory.
    ///
    /// No-op for files — shows a status message instead.
    /// Mutually exclusive with all other special preview modes.
    pub fn toggle_du_preview(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if !entry.is_dir {
                self.status_message = Some("Disk usage view is for directories".to_string());
                return;
            }
        }
        self.du_preview_mode = !self.du_preview_mode;
        if self.du_preview_mode {
            self.hex_view_mode = false;
            self.file_compare_mode = false;
            self.meta_preview_mode = false;
            self.diff_preview_mode = false;
            self.git_log_mode = false;
        }
        self.load_preview();
    }

    /// Build a disk-usage breakdown for `path` using `du -k -d 1`.
    ///
    /// Entries sorted largest-first with human-readable sizes and a
    /// proportional 20-char Unicode block bar.
    pub fn load_du_lines(path: &Path) -> Vec<String> {
        let output = match Command::new("du")
            .args(["-k", "-d", "1"])
            .arg(path)
            .output()
        {
            Ok(out) if out.status.success() => out,
            Ok(out) => {
                return vec![
                    String::new(),
                    format!(
                        "  du failed: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ),
                ]
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return vec![
                    String::new(),
                    "  Disk usage requires 'du' (not found)".to_string(),
                    String::new(),
                    "  du is a POSIX standard tool — check your PATH".to_string(),
                ]
            }
            Err(e) => return vec![String::new(), format!("  Failed to run du: {}", e)],
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let mut entries: Vec<(u64, String)> = text
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, '\t');
                let kb: u64 = parts.next()?.trim().parse().ok()?;
                let full_path = parts.next()?.trim();
                let p = Path::new(full_path);
                if p == path {
                    return None;
                }
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| full_path.to_string());
                Some((kb, name))
            })
            .collect();

        entries.sort_by(|a, b| b.0.cmp(&a.0));

        if entries.is_empty() {
            return vec![String::new(), "  (empty directory)".to_string()];
        }

        let max_kb = entries[0].0.max(1);
        const BAR_WIDTH: usize = 20;
        const BLOCK_CHARS: &[char] = &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

        let mut lines = vec![String::new()];
        for (kb, name) in &entries {
            let human = super::meta_human_size(*kb * 1024);
            let filled_eighths = ((*kb as f64 / max_kb as f64) * BAR_WIDTH as f64 * 8.0) as usize;
            let full_blocks = filled_eighths / 8;
            let partial = filled_eighths % 8;
            let mut bar = String::with_capacity(BAR_WIDTH + 4);
            for _ in 0..full_blocks {
                bar.push('█');
            }
            if partial > 0 && full_blocks < BAR_WIDTH {
                bar.push(BLOCK_CHARS[partial]);
            }
            let remaining = BAR_WIDTH.saturating_sub(full_blocks + if partial > 0 { 1 } else { 0 });
            for _ in 0..remaining {
                bar.push('░');
            }
            lines.push(format!("  {:<30}  {:>10}  {}", name, human, bar));
        }
        lines
    }
}
