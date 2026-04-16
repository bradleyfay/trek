use crate::git::GitStatus;
use crate::highlight::Highlighter;
use crate::ops::Clipboard;
use crate::theme::Theme;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

mod archive_nav;
mod bookmarks;
pub mod change_feed;
mod change_feed_ops;
mod cmux;
mod content;
pub mod context_bundle;
mod file_ops;
mod filter;
mod find;
pub mod frecency;
mod gitignore;
mod layout;
mod metadata;
mod mouse;
pub mod nav_state;
mod navigation;
pub mod opener;
pub mod overlay_state;
pub mod palette;
mod palette_ops;
mod preview;
pub mod preview_state;
mod quick_rename;
mod search;
mod selection;
pub mod session_snapshot;
pub mod session_summary;
mod sort;
pub mod task_manager;
mod task_ops;
mod yank;

/// Maximum directory entries loaded in a single pane.
/// Prevents UI freezes when navigating extremely large directories (e.g. node_modules).
const MAX_ENTRIES: usize = 10_000;

/// Which divider the user is currently dragging.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    LeftDivider,
    RightDivider,
}

/// Maximum number of directory-jump history entries retained per session.
const MAX_HISTORY: usize = 50;

/// One entry in the directory-jump history stack.
pub(super) struct HistoryEntry {
    pub(super) dir: PathBuf,
    pub(super) selected: usize,
}

/// Which field is used to order directory entries.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SortMode {
    #[default]
    Name,
    Size,
    Modified,
    Extension,
}

impl SortMode {
    /// Cycle to the next mode: Name → Size → Modified → Extension → Name.
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Size,
            Self::Size => Self::Modified,
            Self::Modified => Self::Extension,
            Self::Extension => Self::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Size => "Size",
            Self::Modified => "Modified",
            Self::Extension => "Extension",
        }
    }
}

/// Sort direction.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

pub struct App {
    // --- Navigation sub-state ---
    pub nav: nav_state::NavigationState,
    // --- Preview sub-state ---
    pub preview: preview_state::PreviewState,
    // --- Overlay sub-state ---
    pub overlay: overlay_state::OverlayState,

    // --- Terminal dimensions ---
    /// Total height of the terminal (set via apply_layout).
    pub term_height: u16,
    /// Total width of the terminal (set via apply_layout).
    pub term_width: u16,

    // --- Pane layout (percentage-based, 0.0..1.0) ---
    /// Fraction of width where the left divider sits.
    pub left_div: f64,
    /// Fraction of width where the right divider sits.
    pub right_div: f64,

    // --- Left pane collapse (\) ---
    /// True when the left parent-directory pane is collapsed (hidden).
    pub left_collapsed: bool,
    /// Saved `left_div` ratio restored when expanding the pane.
    pub left_div_saved: f64,

    // --- Drag state ---
    pub drag: Option<DragTarget>,

    // --- Pixel positions of dividers (set via apply_layout) ---
    pub left_div_col: u16,
    pub right_div_col: u16,

    /// Areas of each pane (set via apply_layout): (x, y, width, height).
    pub parent_area: (u16, u16, u16, u16),
    pub current_area: (u16, u16, u16, u16),
    pub preview_area: (u16, u16, u16, u16),

    // --- Status message (e.g. "Yanked: ./src/main.rs") ---
    pub status_message: Option<String>,

    // --- Git integration ---
    /// Cached git status for the current repository; None outside of a repo.
    pub git_status: Option<GitStatus>,

    // --- Filesystem watcher (auto-refresh, always-on by default; I toggles off/on) ---
    pub watcher: Option<crate::watcher::DirWatcher>,

    // --- Syntax highlighter (initialized once at startup) ---
    pub highlighter: Highlighter,

    // --- File operations clipboard ---
    pub clipboard: Option<Clipboard>,
    pub pending_delete: Vec<PathBuf>,
    pub pending_extract: Option<PathBuf>,
    pub last_trashed: Vec<crate::trash::TrashedEntry>,

    // --- Async git status ---
    pub git_status_rx: Option<std::sync::mpsc::Receiver<crate::git::GitStatusAsyncResult>>,

    // --- Background file-operation task manager (Ctrl+T) ---
    pub task_manager: task_manager::TaskManager,
    pub(super) task_pending: Vec<task_manager::PendingTask>,

    // --- Live change feed ---
    pub recursive_watcher: Option<crate::watcher::RecursiveWatcher>,
    pub change_feed: change_feed::ChangeFeed,
    pub change_feed_root: PathBuf,

    // --- Archive virtual-filesystem browser ---
    pub archive_path: Option<PathBuf>,
    pub archive_virt_dir: String,
    pub archive_flat_paths: Vec<String>,

    // --- Session change summary (S) ---
    pub session_snapshot: Option<session_snapshot::SessionSnapshot>,
    pub session_summary_cache: Option<Vec<session_snapshot::ChangedFile>>,
    pub session_summary_total: usize,
    pub session_summary_selected: usize,

    // --- Colour theme ---
    pub theme: Theme,
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: std::time::SystemTime,
    /// Number of direct children for directory entries; `None` if unreadable.
    /// Capped at 1001 — a value of 1001 means ">1000 items".
    pub child_count: Option<u32>,
}

impl App {
    pub fn new(start_dir: Option<PathBuf>, theme: Theme) -> Result<Self> {
        let cwd = match start_dir {
            Some(dir) => dir,
            None => std::env::current_dir()?,
        };

        // Start the recursive watcher on cwd immediately so the first frame
        // renders without blocking. The true git repo root (which may be an
        // ancestor of cwd) is resolved asynchronously by load_git_status_async
        // and the watcher is repointed in check_git_status_rx once it arrives.
        let feed_root = cwd.clone();
        let recursive_watcher = crate::watcher::RecursiveWatcher::new(&feed_root);

        let mut app = Self {
            nav: nav_state::NavigationState::new(cwd.clone()),
            preview: preview_state::PreviewState::new(),
            overlay: overlay_state::OverlayState::new(),
            term_height: 0,
            term_width: 0,
            left_div: 0.20,
            right_div: 0.55,
            left_collapsed: false,
            left_div_saved: 0.20,
            drag: None,
            left_div_col: 0,
            right_div_col: 0,
            parent_area: (0, 0, 0, 0),
            current_area: (0, 0, 0, 0),
            preview_area: (0, 0, 0, 0),
            status_message: None,
            git_status: None,
            watcher: crate::watcher::DirWatcher::new(&cwd),
            highlighter: Highlighter::new(),
            clipboard: None,
            pending_delete: Vec::new(),
            pending_extract: None,
            last_trashed: Vec::new(),
            git_status_rx: None,
            recursive_watcher,
            change_feed: change_feed::ChangeFeed::new(),
            change_feed_root: feed_root,
            task_manager: task_manager::TaskManager::new(),
            task_pending: Vec::new(),
            archive_path: None,
            archive_virt_dir: String::new(),
            archive_flat_paths: Vec::new(),
            session_snapshot: None,
            session_summary_cache: None,
            session_summary_total: 0,
            session_summary_selected: 0,
            theme,
        };
        app.load_dir();
        Ok(app)
    }

    /// Invalidate the cached parent directory so the next `load_dir` call
    /// re-reads it from disk.
    ///
    /// Called by `check_watcher` so that watcher-triggered reloads always
    /// produce a fresh parent listing. Filter and sort changes do not call
    /// this — they reuse the cached parent because `cwd` did not change.
    pub fn invalidate_parent_cache(&mut self) {
        self.nav.cached_parent_path = None;
    }

    /// Reload the current directory listing and parent listing.
    ///
    /// Errors are surfaced via `status_message` rather than propagated, so the
    /// app stays alive even if the working directory becomes unreadable.
    pub fn load_dir(&mut self) {
        match Self::read_entries(
            &self.nav.cwd,
            self.nav.show_hidden,
            self.nav.sort_mode,
            self.nav.sort_order,
        ) {
            Ok((entries, truncated)) => {
                self.nav.entries = entries;
                self.nav.entries_truncated = truncated;
            }
            Err(e) => {
                self.nav.entries = Vec::new();
                self.nav.entries_truncated = false;
                self.status_message = Some(format!("Cannot read directory: {e}"));
            }
        }

        // Re-apply active filter to the freshly loaded entries.
        if !self.nav.filter_input.is_empty() {
            let pattern = self.nav.filter_input.to_lowercase();
            self.nav
                .entries
                .retain(|e| e.name.to_lowercase().contains(&pattern));
        }

        // Filter gitignored entries using the cached names from the last async
        // load. The async load (below) will refresh these for the new directory.
        if self.nav.hide_gitignored && !self.nav.gitignored_names.is_empty() {
            self.nav
                .entries
                .retain(|e| !self.nav.gitignored_names.contains(&e.name));
        } else if !self.nav.hide_gitignored {
            self.nav.gitignored_names.clear();
        }

        if self.nav.selected >= self.nav.entries.len() {
            self.nav.selected = self.nav.entries.len().saturating_sub(1);
        }

        // Parent entries (errors here are non-fatal; left pane simply stays empty).
        //
        // Skip the disk read when cwd has not crossed a directory boundary since
        // the last load — only the highlight position may have changed.
        if let Some(parent) = self.nav.cwd.parent() {
            if self.nav.cached_parent_path.as_deref() == Some(parent) {
                // Same parent: just refresh the highlight position.
                self.nav.parent_selected = self
                    .nav
                    .parent_entries
                    .iter()
                    .position(|e| e.path == self.nav.cwd)
                    .unwrap_or(0);
            } else {
                match Self::read_entries(
                    parent,
                    self.nav.show_hidden,
                    self.nav.sort_mode,
                    self.nav.sort_order,
                ) {
                    Ok((entries, _)) => {
                        self.nav.parent_selected = entries
                            .iter()
                            .position(|e| e.path == self.nav.cwd)
                            .unwrap_or(0);
                        self.nav.parent_entries = entries;
                        self.nav.cached_parent_path = Some(parent.to_path_buf());
                    }
                    Err(_) => {
                        self.nav.parent_entries.clear();
                        self.nav.parent_selected = 0;
                        self.nav.cached_parent_path = None;
                    }
                }
            }
        } else {
            self.nav.parent_entries.clear();
            self.nav.parent_selected = 0;
            self.nav.cached_parent_path = None;
        }

        // Reset diff-preview mode on navigation; kick off async git-status load
        // so the UI thread is never blocked by git subprocesses.
        self.preview.diff_preview_mode = false;
        self.load_git_status_async();

        // Replace the watcher so it tracks the current directory.
        // Only recreate it when watching is active (watcher is Some).
        if self.watcher.is_some() {
            self.watcher = crate::watcher::DirWatcher::new(&self.nav.cwd);
        }

        self.load_preview();
    }

    /// Spawn a background thread to load git status and (when `hide_gitignored`
    /// is active) the set of gitignored entry names for `cwd`.
    ///
    /// The result is delivered via `self.git_status_rx` and consumed by
    /// `check_git_status_rx` on the next event-loop tick.  Any previously
    /// in-flight load is cancelled by dropping the old receiver.
    pub fn load_git_status_async(&mut self) {
        let cwd = self.nav.cwd.clone();
        let hide_gitignored = self.nav.hide_gitignored;
        let (tx, rx) = std::sync::mpsc::channel();
        self.git_status_rx = Some(rx);
        std::thread::spawn(move || {
            let status = crate::git::GitStatus::load(&cwd);
            let ignored_names = if hide_gitignored {
                Some(crate::git::load_ignored(&cwd))
            } else {
                None
            };
            let _ = tx.send(crate::git::GitStatusAsyncResult {
                status,
                ignored_names,
            });
        });
    }

    /// Poll the async git-status receiver and apply the result if ready.
    ///
    /// Returns `true` when a result was received so the caller knows a redraw
    /// is warranted.
    pub fn check_git_status_rx(&mut self) -> bool {
        let result = match self.git_status_rx.as_ref() {
            Some(rx) => match rx.try_recv() {
                Ok(r) => r,
                // Result not yet ready — keep the receiver alive.
                Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                // Sender dropped (thread panicked or was cancelled) — clear
                // the receiver so the polling loop does not spin forever.
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.git_status_rx = None;
                    return false;
                }
            },
            None => return false,
        };
        self.git_status_rx = None;
        self.git_status = result.status;

        // If the async load discovered the true git repo root and it differs
        // from the current recursive-watch root, repoint the watcher so the
        // live change feed covers the whole repository tree.
        if let Some(ref status) = self.git_status {
            if status.repo_root != self.change_feed_root {
                self.change_feed_root = status.repo_root.clone();
                self.recursive_watcher =
                    crate::watcher::RecursiveWatcher::new(&self.change_feed_root);
            }
        }

        if let Some(ignored) = result.ignored_names {
            let changed = ignored != self.nav.gitignored_names;
            self.nav.gitignored_names = ignored;
            if changed && self.nav.hide_gitignored {
                // Ignored names changed for this directory; re-filter the
                // listing without spawning another git subprocess.
                self.reapply_gitignore_filter();
            }
        } else if !self.nav.hide_gitignored {
            self.nav.gitignored_names.clear();
        }
        true
    }

    /// Re-read the current directory and re-apply all active filters without
    /// touching git state.  Called when the async gitignored-names result
    /// differs from the cached set so the listing can be updated to match.
    fn reapply_gitignore_filter(&mut self) {
        let (entries, truncated) = match Self::read_entries(
            &self.nav.cwd,
            self.nav.show_hidden,
            self.nav.sort_mode,
            self.nav.sort_order,
        ) {
            Ok(r) => r,
            Err(_) => return,
        };
        self.nav.entries = entries;
        self.nav.entries_truncated = truncated;
        if !self.nav.filter_input.is_empty() {
            let pattern = self.nav.filter_input.to_lowercase();
            self.nav
                .entries
                .retain(|e| e.name.to_lowercase().contains(&pattern));
        }
        if self.nav.hide_gitignored && !self.nav.gitignored_names.is_empty() {
            self.nav
                .entries
                .retain(|e| !self.nav.gitignored_names.contains(&e.name));
        }
        if self.nav.selected >= self.nav.entries.len() {
            self.nav.selected = self.nav.entries.len().saturating_sub(1);
        }
    }

    /// Read and sort directory entries, enforcing MAX_ENTRIES.
    ///
    /// Returns `(entries, truncated)`. On I/O error (e.g. permission denied)
    /// the error is returned to the caller rather than silently swallowed.
    pub fn read_entries(
        dir: &Path,
        show_hidden: bool,
        sort_mode: SortMode,
        sort_order: SortOrder,
    ) -> Result<(Vec<DirEntry>, bool), std::io::Error> {
        let rd = fs::read_dir(dir)?;

        let mut entries: Vec<DirEntry> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if !show_hidden && name.starts_with('.') {
                    return None;
                }
                let meta = e.metadata().ok();
                let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let child_count = if is_dir {
                    fs::read_dir(e.path())
                        .ok()
                        .map(|rd| rd.take(1001).count() as u32)
                } else {
                    None
                };
                Some(DirEntry {
                    name,
                    path: e.path(),
                    is_dir,
                    size,
                    modified,
                    child_count,
                })
            })
            .take(MAX_ENTRIES + 1) // +1 lets us detect truncation
            .collect();

        let truncated = entries.len() > MAX_ENTRIES;
        if truncated {
            entries.truncate(MAX_ENTRIES);
        }

        Self::sort_entries(&mut entries, sort_mode, sort_order);

        Ok((entries, truncated))
    }

    /// Sort a slice of entries in-place. Directories always appear before files.
    ///
    /// Uses `sort_by_cached_key` so any allocating key (e.g. `.to_lowercase()`)
    /// is computed once per element rather than once per comparison, reducing
    /// allocations from O(n log n) to O(n).
    pub fn sort_entries(entries: &mut [DirEntry], mode: SortMode, order: SortOrder) {
        use std::cmp::Reverse;

        // Primary key `!is_dir` keeps directories before files in all modes:
        // `false` (dir) sorts before `true` (file) in ascending order.
        // Secondary keys are wrapped in `Reverse` for descending order while
        // leaving the primary key unaffected.
        match (mode, order) {
            (SortMode::Name, SortOrder::Ascending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
            }
            (SortMode::Name, SortOrder::Descending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, Reverse(e.name.to_lowercase())));
            }
            (SortMode::Size, SortOrder::Ascending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, e.size));
            }
            (SortMode::Size, SortOrder::Descending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, Reverse(e.size)));
            }
            (SortMode::Modified, SortOrder::Ascending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, e.modified));
            }
            (SortMode::Modified, SortOrder::Descending) => {
                entries.sort_by_cached_key(|e| (!e.is_dir, Reverse(e.modified)));
            }
            (SortMode::Extension, SortOrder::Ascending) => {
                entries.sort_by_cached_key(|e| {
                    let ext = Path::new(&e.name)
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    (!e.is_dir, ext, e.name.to_lowercase())
                });
            }
            (SortMode::Extension, SortOrder::Descending) => {
                entries.sort_by_cached_key(|e| {
                    let ext = Path::new(&e.name)
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    (!e.is_dir, Reverse(ext), Reverse(e.name.to_lowercase()))
                });
            }
        }
    }
}

// ── Metadata helpers ──────────────────────────────────────────────────────────

/// Format the 9 permission bits of a Unix mode as `rwxrwxrwx`.
pub fn format_permission_bits(mode: u32) -> String {
    let bit = |shift: u32, c: char| {
        if mode & (1 << shift) != 0 {
            c
        } else {
            '-'
        }
    };
    format!(
        "{}{}{}{}{}{}{}{}{}",
        bit(8, 'r'),
        bit(7, 'w'),
        bit(6, 'x'),
        bit(5, 'r'),
        bit(4, 'w'),
        bit(3, 'x'),
        bit(2, 'r'),
        bit(1, 'w'),
        bit(0, 'x'),
    )
}

/// Human-readable size with one decimal place (B / KB / MB / GB).
pub fn meta_human_size(bytes: u64) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = KB * 1_024;
    const GB: u64 = MB * 1_024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a UNIX timestamp as `YYYY-MM-DD HH:MM:SS` (UTC) without external
/// crates or spawning a subprocess.
pub fn format_unix_timestamp_utc(secs: u64) -> String {
    let (year, month, day, hh, mm, ss) = crate::datetime::decompose_unix_secs(secs);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hh, mm, ss
    )
}

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Format a `SystemTime` as a fixed-12-char listing date (UTC).
///
/// - Same calendar year: `"Jan 15 14:32"` (13 chars including trailing space to reach 12 — actually exactly 12)
/// - Prior year: `"2023 Nov  8 "` (12 chars including trailing space)
/// - Unavailable (epoch = 0): `"----  --:--"` (11 chars, padded to 12 via one leading space)
pub fn format_listing_date(t: std::time::SystemTime) -> String {
    let file_secs = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) if d.as_secs() > 0 => d.as_secs(),
        _ => return "----  --:--".to_string(),
    };
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (fy, fm, fd, fh, fmin, _) = crate::datetime::decompose_unix_secs(file_secs);
    let (ny, ..) = crate::datetime::decompose_unix_secs(now_secs);

    let mon = MONTH_ABBR[fm.saturating_sub(1) as usize];
    if fy == ny {
        // "Jan 15 14:32" — 12 chars
        format!("{} {:2} {:02}:{:02}", mon, fd, fh, fmin)
    } else {
        // "2023 Nov  8 " — 12 chars (trailing space for alignment)
        format!("{} {} {:2} ", fy, mon, fd)
    }
}

/// Format a directory's child item count for display in the listing.
///
/// - `None`: unreadable directory → `"? items"`
/// - `Some(1001)`: sentinel for >1000 → `">1000 items"`
/// - `Some(1)`: singular → `"  1 item"`
/// - Otherwise: `"  N items"` right-aligned in a 3-char number field
pub fn format_dir_count(count: Option<u32>) -> String {
    match count {
        None => "? items".to_string(),
        Some(n) if n >= 1001 => ">1000 items".to_string(),
        Some(1) => "  1 item".to_string(),
        Some(n) => format!("{:>3} items", n),
    }
}

/// Format a file size in human-readable form.
pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Estimate token count from byte size using the bytes/4 heuristic and format for display.
pub fn format_tokens(size: u64) -> String {
    let tokens = size / 4;
    if tokens < 1_000 {
        format!("{} tok", tokens)
    } else if tokens < 1_000_000 {
        format!("{:.1}k tok", tokens as f64 / 1_000.0)
    } else {
        format!("{:.1}M tok", tokens as f64 / 1_000_000.0)
    }
}

/// Simple fuzzy matching: all characters of `query` appear in `name` in order.
pub(super) fn fuzzy_match(name: &str, query: &str) -> bool {
    let mut name_chars = name.chars();
    for qc in query.chars() {
        loop {
            match name_chars.next() {
                Some(nc) if nc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Get the user's home directory without pulling in another crate.
pub(super) fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
