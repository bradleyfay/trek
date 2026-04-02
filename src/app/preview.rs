use super::{App, SortMode, SortOrder};
use crate::icons::icon_for_entry;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, LazyLock};

/// The hex dump tool available on this system, probed exactly once per session.
///
/// Tries `xxd` first, then `hexdump`. Returns `None` if neither is available.
/// Probes by attempting to spawn the binary directly (no `which`), so it works
/// even when `which` is absent. Exit code is intentionally ignored — only spawn
/// success matters. All stdio is suppressed to prevent terminal pollution.
static HEX_TOOL: LazyLock<Option<(&'static str, &'static [&'static str])>> = LazyLock::new(|| {
    let probe = |bin: &str, probe_arg: &str| -> bool {
        // Exit code is ignored; we only care whether the binary can be spawned.
        Command::new(bin)
            .arg(probe_arg)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    };
    if probe("xxd", "--version") {
        Some(("xxd", &[] as &[&str]))
    } else if probe("hexdump", "--version") {
        Some(("hexdump", &["-C"] as &[&str]))
    } else {
        None
    }
});

/// The result of an async preview computation.
pub struct PreviewResult {
    pub lines: Vec<String>,
    pub is_diff: bool,
}

/// Describes what a preview thread should compute.
///
/// Built synchronously from app state in `build_preview_job`, then executed
/// on a background thread so navigation is never blocked.
enum PreviewJob {
    Empty,
    HexDump {
        path: PathBuf,
    },
    FileDiff {
        left: PathBuf,
        right: PathBuf,
    },
    Meta {
        path: PathBuf,
    },
    GitLog {
        path: PathBuf,
    },
    DiskUsage {
        path: PathBuf,
    },
    GitDiff {
        path: PathBuf,
        has_change: bool,
    },
    FileContent {
        path: PathBuf,
    },
    ImagePreview {
        path: PathBuf,
    },
    PdfPreview {
        path: PathBuf,
    },
    DirectoryListing {
        path: PathBuf,
        show_hidden: bool,
        sort_mode: SortMode,
        sort_order: SortOrder,
    },
}

impl PreviewJob {
    fn execute(self) -> PreviewResult {
        match self {
            PreviewJob::Empty => PreviewResult {
                lines: Vec::new(),
                is_diff: false,
            },
            PreviewJob::HexDump { path } => PreviewResult {
                lines: App::load_hex_lines(&path),
                is_diff: false,
            },
            PreviewJob::FileDiff { left, right } => PreviewResult {
                lines: App::load_file_diff_static(&left, &right),
                is_diff: true,
            },
            PreviewJob::Meta { path } => PreviewResult {
                lines: App::load_meta_lines(&path),
                is_diff: false,
            },
            PreviewJob::GitLog { path } => PreviewResult {
                lines: App::load_git_log_static(&path),
                is_diff: false,
            },
            PreviewJob::DiskUsage { path } => PreviewResult {
                lines: App::load_du_lines(&path),
                is_diff: false,
            },
            PreviewJob::GitDiff { path, has_change } => {
                if has_change {
                    let diff = App::load_git_diff_static(&path);
                    if !diff.is_empty() {
                        return PreviewResult {
                            lines: diff,
                            is_diff: true,
                        };
                    }
                }
                // No diff — fall through to raw content.
                PreviewResult {
                    lines: App::read_file_preview(&path),
                    is_diff: false,
                }
            }
            PreviewJob::FileContent { path } => PreviewResult {
                lines: App::read_file_preview(&path),
                is_diff: false,
            },
            PreviewJob::ImagePreview { path } => PreviewResult {
                lines: build_image_preview_lines(&path),
                is_diff: false,
            },
            PreviewJob::PdfPreview { path } => PreviewResult {
                lines: build_pdf_preview_lines(&path),
                is_diff: false,
            },
            PreviewJob::DirectoryListing {
                path,
                show_hidden,
                sort_mode,
                sort_order,
            } => {
                let lines = App::read_entries(&path, show_hidden, sort_mode, sort_order)
                    .map(|(children, _)| {
                        children
                            .iter()
                            .map(|c| {
                                let icon = icon_for_entry(&c.name, c.is_dir);
                                format!("{} {}", icon, c.name)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                PreviewResult {
                    lines,
                    is_diff: false,
                }
            }
        }
    }
}

impl App {
    /// Build the preview job for the current app state without doing any I/O.
    fn build_preview_job(&self) -> PreviewJob {
        // Hex dump — first priority.
        if self.hex_view_mode {
            if let Some(entry) = self.entries.get(self.selected) {
                if !entry.is_dir {
                    return PreviewJob::HexDump {
                        path: entry.path.clone(),
                    };
                }
            }
            return PreviewJob::Empty;
        }

        // File compare — requires exactly 2 non-directory files selected.
        if self.file_compare_mode {
            let paths: Vec<PathBuf> = self
                .selection
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .filter(|e| !e.is_dir)
                .map(|e| e.path.clone())
                .collect();
            if paths.len() == 2 {
                return PreviewJob::FileDiff {
                    left: paths[0].clone(),
                    right: paths[1].clone(),
                };
            }
            return PreviewJob::Empty;
        }

        // Metadata card.
        if self.meta_preview_mode {
            if let Some(entry) = self.entries.get(self.selected) {
                return PreviewJob::Meta {
                    path: entry.path.clone(),
                };
            }
            return PreviewJob::Empty;
        }

        // Git log.
        if self.git_log_mode {
            if let Some(entry) = self.entries.get(self.selected) {
                return PreviewJob::GitLog {
                    path: entry.path.clone(),
                };
            }
            return PreviewJob::Empty;
        }

        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                if self.du_preview_mode {
                    return PreviewJob::DiskUsage {
                        path: entry.path.clone(),
                    };
                }
                return PreviewJob::DirectoryListing {
                    path: entry.path.clone(),
                    show_hidden: self.show_hidden,
                    sort_mode: self.sort_mode,
                    sort_order: self.sort_order,
                };
            } else if self.diff_preview_mode {
                let has_change = self
                    .git_status
                    .as_ref()
                    .and_then(|g| g.for_path(&entry.path))
                    .is_some();
                return PreviewJob::GitDiff {
                    path: entry.path.clone(),
                    has_change,
                };
            } else if is_image_path(&entry.path) {
                return PreviewJob::ImagePreview {
                    path: entry.path.clone(),
                };
            } else if is_pdf_path(&entry.path) {
                return PreviewJob::PdfPreview {
                    path: entry.path.clone(),
                };
            } else {
                return PreviewJob::FileContent {
                    path: entry.path.clone(),
                };
            }
        }

        PreviewJob::Empty
    }

    /// Kick off an async preview render for the currently selected entry.
    ///
    /// Returns immediately — the UI renders a "Loading…" placeholder until
    /// the background thread delivers the result via [`check_preview_rx`].
    ///
    /// Any in-flight render from a previous call is cancelled implicitly: when
    /// the old [`Receiver`] is dropped the background thread's next `tx.send()`
    /// returns `SendError` and the thread exits.
    pub fn load_preview(&mut self) {
        // Drop old receiver → cancels any in-flight thread.
        self.preview_rx = None;
        self.preview_scroll = 0;
        self.preview_lines.clear();
        self.preview_is_diff = false;
        // Reset focus state whenever the previewed file changes.
        self.preview_focused = false;
        self.preview_cursor = 0;
        self.preview_selection_anchor = None;

        let job = self.build_preview_job();
        if matches!(job, PreviewJob::Empty) {
            self.preview_loading = false;
            return;
        }

        self.preview_loading = true;
        let (tx, rx) = mpsc::channel::<PreviewResult>();
        self.preview_rx = Some(rx);

        std::thread::spawn(move || {
            let result = job.execute();
            // Ignore SendError — means the caller already moved to another file.
            let _ = tx.send(result);
        });
    }

    /// Poll the async preview channel and apply any pending result.
    ///
    /// Must be called on every event-loop iteration so the UI stays live.
    pub fn check_preview_rx(&mut self) {
        let result = match self.preview_rx.as_ref() {
            Some(rx) => rx.try_recv().ok(),
            None => return,
        };
        if let Some(result) = result {
            self.preview_lines = result.lines;
            self.preview_is_diff = result.is_diff;
            self.preview_loading = false;
            self.preview_rx = None;
        }
    }

    /// Load `git diff` (unstaged, then staged) for `path` as a list of lines.
    pub(super) fn load_git_diff_static(path: &Path) -> Vec<String> {
        crate::git::diff_for_preview(path)
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

    // ── Preview focus mode ────────────────────────────────────────────────────

    /// Enter preview focus mode. The cursor starts at the current top-visible line.
    pub fn enter_preview_focus(&mut self) {
        self.preview_focused = true;
        self.preview_cursor = self.preview_scroll;
        self.preview_selection_anchor = None;
    }

    /// Exit preview focus mode and return focus to the file tree.
    pub fn exit_preview_focus(&mut self) {
        self.preview_focused = false;
        self.preview_selection_anchor = None;
    }

    /// Move the preview cursor down one line, scrolling if necessary.
    pub fn preview_cursor_down(&mut self) {
        if self.preview_lines.is_empty() {
            return;
        }
        let max = self.preview_lines.len().saturating_sub(1);
        if self.preview_cursor < max {
            self.preview_cursor += 1;
            self.ensure_preview_cursor_visible();
        }
    }

    /// Move the preview cursor up one line, scrolling if necessary.
    pub fn preview_cursor_up(&mut self) {
        if self.preview_cursor > 0 {
            self.preview_cursor -= 1;
            self.ensure_preview_cursor_visible();
        }
    }

    /// Extend selection downward: set anchor if not yet set, then move cursor down.
    pub fn preview_select_down(&mut self) {
        if self.preview_selection_anchor.is_none() {
            self.preview_selection_anchor = Some(self.preview_cursor);
        }
        self.preview_cursor_down();
    }

    /// Extend selection upward: set anchor if not yet set, then move cursor up.
    pub fn preview_select_up(&mut self) {
        if self.preview_selection_anchor.is_none() {
            self.preview_selection_anchor = Some(self.preview_cursor);
        }
        self.preview_cursor_up();
    }

    /// Adjust `preview_scroll` so `preview_cursor` stays within the visible area.
    pub(crate) fn ensure_preview_cursor_visible(&mut self) {
        let visible = if self.preview_area.3 > 2 {
            (self.preview_area.3 - 2) as usize
        } else {
            40
        };
        if self.preview_cursor < self.preview_scroll {
            self.preview_scroll = self.preview_cursor;
        } else if self.preview_cursor >= self.preview_scroll + visible {
            self.preview_scroll = self.preview_cursor + 1 - visible;
        }
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
    /// Requires exactly 2 non-directory entries in `selection`.
    /// Mutually exclusive with all other special preview modes.
    pub fn toggle_file_compare(&mut self) {
        let any_dir = self
            .selection
            .iter()
            .any(|&i| self.entries.get(i).map(|e| e.is_dir).unwrap_or(false));
        if any_dir {
            self.status_message = Some("File comparison not available for directories".to_string());
            return;
        }
        if self.selection.len() != 2 {
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

        let (cmd, args): (&str, &[&str]) = match *HEX_TOOL {
            Some((cmd, args)) => (cmd, args),
            None => {
                return vec![
                    String::new(),
                    "  Hex view requires xxd or hexdump".to_string(),
                    String::new(),
                    "  Install: brew install vim   (macOS, provides xxd)".to_string(),
                    "           apt install xxd    (Debian/Ubuntu)".to_string(),
                ];
            }
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
    pub(super) fn load_file_diff_static(a: &Path, b: &Path) -> Vec<String> {
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
        self.load_git_status_async();
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
    pub(super) fn load_git_log_static(path: &Path) -> Vec<String> {
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

// ── Image / PDF preview helpers ───────────────────────────────────────────

/// Returns `true` when `path` has a binary raster image extension.
///
/// SVG is intentionally excluded — it is plain-text XML and previews fine
/// through the normal text path.
fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "avif" | "tiff" | "tif")
    )
}

/// Returns `true` when `path` has the `.pdf` extension.
fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// Build preview lines for a raster image file.
///
/// Shows format, pixel dimensions (via `imagesize`), file size, and — when
/// `chafa` is available on `$PATH` — a Unicode/sixel art rendering of the
/// image at the preview pane's approximate width (72 columns).
fn build_image_preview_lines(path: &Path) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Format label from extension.
    let fmt = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("image")
        .to_ascii_uppercase();

    // File size.
    let size_str = std::fs::metadata(path)
        .map(|m| human_bytes(m.len()))
        .unwrap_or_else(|_| "?".to_string());

    // Pixel dimensions via imagesize (reads just the header, not the whole file).
    let dim_str = match imagesize::size(path) {
        Ok(dim) => format!("{} × {}", dim.width, dim.height),
        Err(_) => "unknown dimensions".to_string(),
    };

    lines.push(format!("  Format    {}", fmt));
    lines.push(format!("  Size      {}", size_str));
    lines.push(format!("  Dimensions  {}", dim_str));
    lines.push(String::new());

    // Try chafa for a visual text rendering. Graceful no-op if not installed.
    let chafa_out = std::process::Command::new("chafa")
        .args([
            "--size=72x36",
            "--colors=256",
            "--animate=off", // prevent animated GIF corruption
            "--",
            &path.to_string_lossy(),
        ])
        .output();
    if let Ok(out) = chafa_out {
        if out.status.success() && !out.stdout.is_empty() {
            let rendered = String::from_utf8_lossy(&out.stdout);
            lines.extend(rendered.lines().map(|l| l.to_string()));
            return lines;
        }
    }

    // No chafa — show a hint.
    lines.push("  [install chafa for inline image preview]".to_string());
    lines
}

/// Build preview lines for a PDF file.
///
/// Shows file size and PDF version from the header.  When `pdfinfo` (from
/// poppler-utils) is available it is used for richer metadata; otherwise a
/// concise header-only summary is returned.
fn build_pdf_preview_lines(path: &Path) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    let size_str = std::fs::metadata(path)
        .map(|m| human_bytes(m.len()))
        .unwrap_or_else(|_| "?".to_string());

    // Read PDF version from the `%PDF-x.y` header (first 16 bytes).
    let version = std::fs::File::open(path)
        .ok()
        .and_then(|mut f| {
            use std::io::Read;
            let mut buf = [0u8; 16];
            f.read_exact(&mut buf).ok()?;
            let header = std::str::from_utf8(&buf).ok()?;
            let v = header.strip_prefix("%PDF-")?;
            Some(v.split_whitespace().next().unwrap_or("").to_string())
        })
        .unwrap_or_default();

    lines.push("  Format    PDF".to_string());
    if !version.is_empty() {
        lines.push(format!("  Version   {}", version));
    }
    lines.push(format!("  Size      {}", size_str));
    lines.push(String::new());

    // Try pdfinfo for rich metadata.
    let pdfinfo = std::process::Command::new("pdfinfo")
        .arg(path.to_string_lossy().as_ref())
        .output();
    if let Ok(out) = pdfinfo {
        if out.status.success() && !out.stdout.is_empty() {
            let info = String::from_utf8_lossy(&out.stdout);
            lines.extend(info.lines().map(|l| format!("  {}", l)));
            return lines;
        }
    }

    lines.push("  [install pdfinfo (poppler-utils) for detailed metadata]".to_string());
    lines
}

/// Format a byte count as a human-readable string (e.g. "1.2 MB").
fn human_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if n >= GB {
        format!("{:.1} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Given: a path with a .png extension
    /// When: is_image_path is called
    /// Then: returns true
    #[test]
    fn is_image_path_recognises_png() {
        assert!(is_image_path(&PathBuf::from("photo.png")));
    }

    /// Given: a path with a .jpg extension
    /// When: is_image_path is called
    /// Then: returns true
    #[test]
    fn is_image_path_recognises_jpg() {
        assert!(is_image_path(&PathBuf::from("photo.jpg")));
    }

    /// Given: a path with a .svg extension
    /// When: is_image_path is called
    /// Then: returns false (SVG is text)
    #[test]
    fn is_image_path_excludes_svg() {
        assert!(!is_image_path(&PathBuf::from("icon.svg")));
    }

    /// Given: a path with a .pdf extension
    /// When: is_pdf_path is called
    /// Then: returns true
    #[test]
    fn is_pdf_path_recognises_pdf() {
        assert!(is_pdf_path(&PathBuf::from("doc.pdf")));
    }

    /// Given: a path with a .txt extension
    /// When: is_pdf_path is called
    /// Then: returns false
    #[test]
    fn is_pdf_path_rejects_txt() {
        assert!(!is_pdf_path(&PathBuf::from("readme.txt")));
    }

    /// Given: a valid PDF header in a temp file
    /// When: build_pdf_preview_lines is called
    /// Then: lines mention "PDF" and do not contain "[binary file]"
    #[test]
    fn build_pdf_preview_lines_shows_pdf_format() {
        let tmp = std::env::temp_dir().join(format!("trek_pdftest_{}", std::process::id()));
        std::fs::write(&tmp, b"%PDF-1.4\n%%EOF\n").unwrap();
        let lines = build_pdf_preview_lines(&tmp);
        let joined = lines.join("\n");
        assert!(joined.contains("PDF"), "expected PDF in: {joined}");
        assert!(
            !joined.contains("[binary file]"),
            "unexpected binary placeholder: {joined}"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    /// Given: a PNG file with a valid header
    /// When: build_image_preview_lines is called
    /// Then: lines mention "PNG" and do not contain "[binary file]"
    #[test]
    fn build_image_preview_lines_shows_png_format() {
        let tmp = std::env::temp_dir().join(format!("trek_pngtest_{}", std::process::id()));
        // Minimal PNG header (signature + partial IHDR — enough for imagesize)
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53,
        ];
        // Write with .png extension so is_image_path works
        let png_path = tmp.with_extension("png");
        std::fs::write(&png_path, png_bytes).unwrap();
        let lines = build_image_preview_lines(&png_path);
        let joined = lines.join("\n");
        assert!(joined.contains("PNG"), "expected PNG in: {joined}");
        assert!(
            !joined.contains("[binary file]"),
            "unexpected binary placeholder: {joined}"
        );
        let _ = std::fs::remove_file(&png_path);
    }

    /// Given: a byte count in the GB range
    /// When: human_bytes is called
    /// Then: returns a GB-suffixed string
    #[test]
    fn human_bytes_gb() {
        let s = human_bytes(2 * 1024 * 1024 * 1024);
        assert!(s.ends_with(" GB"), "expected GB suffix, got: {s}");
    }

    /// Given: a byte count in the KB range
    /// When: human_bytes is called
    /// Then: returns a KB-suffixed string
    #[test]
    fn human_bytes_kb() {
        let s = human_bytes(2048);
        assert!(s.ends_with(" KB"), "expected KB suffix, got: {s}");
    }
}
