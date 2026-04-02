use crate::git::GitStatus;
use crate::highlight::Highlighter;
use crate::ops::Clipboard;
use crate::search::SearchResultGroup;
use anyhow::Result;
use frecency::FrecencyEntry;
use std::collections::{HashMap, HashSet};
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
mod navigation;
pub mod opener;
pub mod palette;
mod palette_ops;
mod preview;
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
    /// Current directory being browsed.
    pub cwd: PathBuf,
    /// Sorted entries in the current directory.
    pub entries: Vec<DirEntry>,
    /// True when the entry list was truncated to MAX_ENTRIES.
    pub entries_truncated: bool,
    /// Index of the selected entry.
    pub selected: usize,
    /// Scroll offset for the current pane.
    pub current_scroll: usize,
    /// Entries in the parent directory (for left pane).
    pub parent_entries: Vec<DirEntry>,
    /// Index of cwd within its parent listing.
    pub parent_selected: usize,
    /// Scroll offset for the parent pane.
    pub parent_scroll: usize,
    /// Lines of the previewed file (right pane).
    pub preview_lines: Vec<String>,
    /// Preview scroll offset (line index of top visible line).
    pub preview_scroll: usize,
    /// Total height of the terminal (set via apply_layout).
    pub term_height: u16,
    /// Total width of the terminal (set via apply_layout).
    pub term_width: u16,

    // --- Pane layout (percentage-based, 0.0..1.0) ---
    /// Fraction of width where the left divider sits.
    pub left_div: f64,
    /// Fraction of width where the right divider sits.
    pub right_div: f64,

    // --- Preview pane collapse (w) ---
    /// True when the right preview pane is collapsed (hidden).
    pub preview_collapsed: bool,
    /// Saved `right_div` ratio restored when expanding the pane.
    pub preview_right_div: f64,

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

    // --- Fuzzy search ---
    pub search_mode: bool,
    pub search_query: String,
    /// Indices into `entries` that match the current query.
    pub filtered_indices: Vec<usize>,
    /// O(1) membership check for filtered indices.
    pub filtered_set: HashSet<usize>,
    /// Selection before search started (for cancel-restore).
    pub pre_search_selected: usize,

    // --- Status message (e.g. "Yanked: ./src/main.rs") ---
    pub status_message: Option<String>,

    // --- Hidden files toggle ---
    pub show_hidden: bool,

    // --- Help overlay ---
    pub show_help: bool,

    // --- Git integration ---
    /// Cached git status for the current repository; None outside of a repo.
    pub git_status: Option<GitStatus>,
    /// When true the preview pane shows `git diff` output instead of raw file content.
    pub diff_preview_mode: bool,
    /// True when `preview_lines` holds diff output (used by the renderer to colorise lines).
    pub preview_is_diff: bool,

    // --- Metadata preview (m) ---
    /// When true the preview pane shows the file metadata card instead of content.
    pub meta_preview_mode: bool,

    // --- Git log preview (V) ---
    /// When true the preview pane shows `git log --oneline -30 -- <path>`.
    /// Mutually exclusive with diff_preview_mode, meta_preview_mode, hash_preview_mode.
    pub git_log_mode: bool,

    // --- Two-file compare (f) ---
    /// When true the preview pane shows a unified diff of the two selected files.
    /// Mutually exclusive with all other special preview modes.
    pub file_compare_mode: bool,

    // --- Hex dump view (a) ---
    /// When true the preview pane shows a hex dump (xxd / hexdump -C).
    pub hex_view_mode: bool,

    // --- Disk usage preview (D) ---
    /// When true the preview pane shows a `du -k -d 1` breakdown of the selected directory.
    pub du_preview_mode: bool,

    // --- Filesystem watcher (auto-refresh, always-on by default; I toggles off/on) ---
    /// OS-native directory watcher. Some when watching is active, None when
    /// the user has disabled auto-refresh with `I`. Trek starts watching
    /// automatically — no keypress required.
    pub watcher: Option<crate::watcher::DirWatcher>,

    // --- chmod editor (P) ---
    /// True while the chmod input bar is open.
    pub chmod_mode: bool,
    /// Octal string currently typed in the chmod bar (max 4 chars).
    pub chmod_input: String,

    // --- Syntax highlighter (initialized once at startup) ---
    pub highlighter: Highlighter,

    // --- File operations clipboard ---
    /// Files queued for copy or cut.
    pub clipboard: Option<Clipboard>,
    /// Paths pending deletion (non-empty while confirmation prompt is shown).
    pub pending_delete: Vec<PathBuf>,
    /// Archive path awaiting extraction confirmation (Some while prompt is shown).
    pub pending_extract: Option<PathBuf>,
    /// The most recent group of trashed items, available for undo with `u`.
    pub last_trashed: Vec<crate::trash::TrashedEntry>,
    /// True while the mkdir name input bar is open.
    pub mkdir_mode: bool,
    /// Name typed by the user in mkdir mode.
    pub mkdir_input: String,

    // --- Content search (Ctrl+F / rg) ---
    /// True while the content search prompt is open.
    pub content_search_mode: bool,
    /// Query string typed by the user.
    pub content_search_query: String,
    /// Grouped results from the last rg run.
    pub content_search_results: Vec<SearchResultGroup>,
    /// Flat index into the flattened match list (for j/k navigation).
    pub content_search_selected: usize,
    /// Error message shown when rg is missing or returns an error.
    pub content_search_error: Option<String>,
    /// True when results have been truncated at MAX_RESULTS.
    pub content_search_truncated: bool,

    // --- Multi-file selection (Space / J / K / v) ---
    /// Indices into `entries` that the user has marked (for copy, delete, etc.).
    pub rename_selected: HashSet<usize>,

    // --- Sort ---
    /// Current sort field.
    pub sort_mode: SortMode,
    /// Current sort direction.
    pub sort_order: SortOrder,

    // --- Bookmarks (b / B) ---
    /// True while the bookmark picker overlay is open.
    pub bookmark_mode: bool,
    /// All bookmarks loaded from disk when the picker opens.
    pub bookmarks: Vec<PathBuf>,
    /// Index into `bookmark_filtered` of the highlighted row.
    pub bookmark_selected: usize,
    /// Filter string typed while the picker is open.
    pub bookmark_query: String,
    /// Indices into `bookmarks` that pass the current filter.
    pub bookmark_filtered: Vec<usize>,

    // --- Recursive find (Ctrl+P) ---
    /// True while the recursive filename find overlay is open.
    pub find_mode: bool,
    /// Query string typed by the user.
    pub find_query: String,
    /// Results from the last find run.
    pub find_results: Vec<crate::find::FindResult>,
    /// Index of the currently highlighted result.
    pub find_selected: usize,
    /// Error message shown when the search fails.
    pub find_error: Option<String>,
    /// True when results were capped at MAX_FIND_RESULTS.
    pub find_truncated: bool,

    // --- Directory jump history ---
    /// Chronological list of visited locations; `history[history_pos]` is current.
    pub(super) history: Vec<HistoryEntry>,
    /// Index into `history` pointing at the current location.
    pub(super) history_pos: usize,

    // --- Filter / narrow mode (|) ---
    /// True while the filter input bar is open (user is actively typing).
    pub filter_mode: bool,
    /// Active filter string. Empty = no filter. Non-empty while filter_mode is false
    /// means the filter is "frozen" (bar closed, listing still narrowed).
    pub filter_input: String,

    // --- Command palette (:) ---
    /// True when the command palette overlay is open.
    pub palette_mode: bool,
    /// The text the user has typed into the palette search bar.
    pub palette_query: String,
    /// Index within `palette_filtered` of the highlighted row.
    pub palette_selected: usize,
    /// Indices into `palette::PALETTE_ACTIONS` matching the current query.
    pub palette_filtered: Vec<usize>,

    // --- Quick single-file rename (n / F2) ---
    /// True when the quick rename bar is open.
    pub quick_rename_mode: bool,
    /// The text currently in the quick rename input bar.
    pub quick_rename_input: String,

    // --- Gitignore-aware listing (i) ---
    /// When true, gitignored entries are hidden from the current listing.
    /// Persists across directory navigation for the session (like show_hidden).
    pub hide_gitignored: bool,
    /// Cached set of gitignored entry names for the current directory.
    /// Populated by load_dir() when hide_gitignored is true.
    pub gitignored_names: std::collections::HashSet<String>,

    // --- Path jump bar (e) ---
    /// True while the path jump input bar is open.
    pub path_mode: bool,
    /// The path string the user is typing in the path jump bar.
    pub path_input: String,

    // --- Touch / new file (t) ---
    /// True while the touch filename input bar is open.
    pub touch_mode: bool,
    /// Filename typed by the user in touch mode.
    pub touch_input: String,

    // --- Preview line numbers (#) ---
    /// When true, each preview line is prefixed with its 1-based line number.
    pub show_line_numbers: bool,

    // --- Preview word wrap (U) ---
    /// When true, the preview pane soft-wraps long lines at the pane boundary.
    pub preview_wrap: bool,

    // --- Directory item counts (N) ---
    /// When true, directory entries show child item counts instead of block sizes.
    pub show_dir_counts: bool,

    // --- Listing timestamps (T) ---
    /// When true, the listing shows last-modified dates instead of file sizes.
    pub show_timestamps: bool,

    // --- Async preview ---
    /// True while a background thread is rendering the preview.
    /// The UI shows a "Loading…" placeholder until the result arrives.
    pub preview_loading: bool,
    /// Receive end of the async preview channel. `None` when no render is in
    /// flight. Dropping this receiver cancels the background thread implicitly.
    pub preview_rx: Option<std::sync::mpsc::Receiver<crate::app::preview::PreviewResult>>,

    // --- Clipboard inspector (F) ---
    /// True while the clipboard contents overlay is open.
    pub clipboard_inspect_mode: bool,

    // --- Symlink creation (L) ---
    /// True while the symlink name input bar is open.
    pub symlink_mode: bool,
    /// Name typed by the user (the link name, not the target).
    pub symlink_input: String,
    /// Absolute path the symlink will point to. Set on begin, cleared on confirm/cancel.
    pub symlink_target: Option<PathBuf>,

    // --- Per-session marks (` to set, ' to jump) ---
    /// True while Trek is waiting for the mark-slot key after the user pressed `.
    pub mark_set_mode: bool,
    /// True while Trek is waiting for the jump-slot key after the user pressed '.
    pub mark_jump_mode: bool,
    /// Maps mark characters (a–z, A–Z) to directory paths recorded this session.
    pub marks: HashMap<char, PathBuf>,

    // --- Yank picker (A) ---
    /// True while the yank format picker overlay is open.
    pub yank_picker_mode: bool,

    // --- AI context bundle (Ctrl+B) ---
    /// True while the context bundle format picker overlay is open.
    pub context_bundle_picker_mode: bool,
    /// True while waiting for the user to confirm copying an oversized bundle.
    pub context_bundle_confirm_mode: bool,
    /// The assembled bundle string awaiting confirmation (> 512 KB).
    pub context_bundle_pending: Option<String>,

    // --- File duplication (W) ---
    /// True while the duplicate name input bar is open.
    pub dup_mode: bool,
    /// Name typed by the user (pre-filled with the suggested name).
    pub dup_input: String,
    /// Source path being duplicated; set when dup_mode is entered.
    pub dup_src: Option<PathBuf>,

    // --- Frecency jump (z) ---
    /// True while the frecency overlay is open.
    pub frecency_mode: bool,
    /// All recorded frecency entries (unsorted, session-scoped).
    pub frecency_list: Vec<FrecencyEntry>,
    /// Indices into `frecency_list` after filter + sort, for display.
    pub frecency_filtered: Vec<usize>,
    /// Cursor row in the frecency overlay.
    pub frecency_selected: usize,
    /// Current filter query typed in the overlay.
    pub frecency_query: String,

    // --- Double-click detection ---
    /// Timestamp of the most recent left-button click.  Used with
    /// `last_click_pos` to detect double-clicks; crossterm does not emit
    /// native double-click events.
    pub last_click_time: Option<std::time::Instant>,
    /// Terminal cell position (col, row) of the most recent left-button click.
    pub last_click_pos: Option<(u16, u16)>,

    // --- Live change feed (F) ---
    /// Recursive watcher watching the project root for all filesystem events.
    /// Feeds events exclusively into `change_feed`. Separate from the
    /// non-recursive `watcher` that triggers directory-listing refresh.
    pub recursive_watcher: Option<crate::watcher::RecursiveWatcher>,
    /// Buffer of recent filesystem events fed by the recursive watcher.
    pub change_feed: change_feed::ChangeFeed,
    /// True while the change feed pane is open (replaces preview pane area).
    pub change_feed_mode: bool,
    /// Root directory being watched recursively. Used to compute relative paths.
    pub change_feed_root: PathBuf,

    // --- Background file-operation task manager (Ctrl+T) ---
    /// Tracks all background file operation tasks (copy, move, extract).
    pub task_manager: task_manager::TaskManager,
    /// True while the task manager overlay is visible.
    pub task_manager_mode: bool,
    /// In-flight background tasks awaiting results via their channels.
    pub(super) task_pending: Vec<task_manager::PendingTask>,

    // --- Archive virtual-filesystem browser ---
    /// True while browsing the contents of an archive as a virtual directory.
    pub archive_mode: bool,
    /// Path to the archive file currently being browsed; `None` outside archive mode.
    pub archive_path: Option<PathBuf>,
    /// Current virtual directory path within the archive (e.g. `"src/utils/"`).
    /// Empty string means the archive root.
    pub archive_virt_dir: String,
    /// Flat list of all entry paths within the archive; populated on first load.
    pub archive_flat_paths: Vec<String>,

    // --- Session change summary (S) ---
    /// Filesystem snapshot taken when session summary mode was last reset, or on
    /// first open if never explicitly reset. `None` until the first `S` press.
    pub session_snapshot: Option<session_snapshot::SessionSnapshot>,
    /// True while the session summary pane is open (replaces the center pane).
    pub session_summary_mode: bool,
    /// Cached diff result; `None` means stale and must be recomputed on next open.
    pub session_summary_cache: Option<Vec<session_snapshot::ChangedFile>>,
    /// Total number of changed files from the last diff (may exceed cache length).
    pub session_summary_total: usize,
    /// Cursor index within the session summary list.
    pub session_summary_selected: usize,
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
    pub fn new(start_dir: Option<PathBuf>) -> Result<Self> {
        let cwd = match start_dir {
            Some(dir) => dir,
            None => std::env::current_dir()?,
        };

        // Determine the recursive-watch root: git repo root, or cwd as fallback.
        let feed_root = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&cwd)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let s = String::from_utf8(o.stdout).ok()?;
                    Some(PathBuf::from(s.trim()))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| cwd.clone());

        let recursive_watcher = crate::watcher::RecursiveWatcher::new(&feed_root);

        let mut app = Self {
            cwd: cwd.clone(),
            entries: Vec::new(),
            entries_truncated: false,
            selected: 0,
            current_scroll: 0,
            parent_entries: Vec::new(),
            parent_selected: 0,
            parent_scroll: 0,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            term_height: 0,
            term_width: 0,
            left_div: 0.20,
            right_div: 0.55,
            preview_collapsed: false,
            preview_right_div: 0.55,
            left_collapsed: false,
            left_div_saved: 0.20,
            drag: None,
            left_div_col: 0,
            right_div_col: 0,
            parent_area: (0, 0, 0, 0),
            current_area: (0, 0, 0, 0),
            preview_area: (0, 0, 0, 0),
            search_mode: false,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            filtered_set: HashSet::new(),
            pre_search_selected: 0,
            status_message: None,
            show_hidden: false,
            show_help: false,
            git_status: None,
            diff_preview_mode: false,
            preview_is_diff: false,
            meta_preview_mode: false,
            git_log_mode: false,
            file_compare_mode: false,
            hex_view_mode: false,
            du_preview_mode: false,
            watcher: crate::watcher::DirWatcher::new(&cwd),
            chmod_mode: false,
            chmod_input: String::new(),
            highlighter: Highlighter::new(),
            clipboard: None,
            pending_delete: Vec::new(),
            pending_extract: None,
            last_trashed: Vec::new(),
            mkdir_mode: false,
            mkdir_input: String::new(),
            content_search_mode: false,
            content_search_query: String::new(),
            content_search_results: Vec::new(),
            content_search_selected: 0,
            content_search_error: None,
            content_search_truncated: false,
            rename_selected: HashSet::new(),
            sort_mode: SortMode::default(),
            sort_order: SortOrder::default(),
            bookmark_mode: false,
            bookmarks: Vec::new(),
            bookmark_selected: 0,
            bookmark_query: String::new(),
            bookmark_filtered: Vec::new(),
            find_mode: false,
            find_query: String::new(),
            find_results: Vec::new(),
            find_selected: 0,
            find_error: None,
            find_truncated: false,
            history: vec![HistoryEntry {
                dir: cwd.clone(),
                selected: 0,
            }],
            history_pos: 0,
            filter_mode: false,
            filter_input: String::new(),
            palette_mode: false,
            palette_query: String::new(),
            palette_selected: 0,
            palette_filtered: palette::filter_palette(""),
            quick_rename_mode: false,
            quick_rename_input: String::new(),
            hide_gitignored: false,
            gitignored_names: std::collections::HashSet::new(),
            path_mode: false,
            path_input: String::new(),
            touch_mode: false,
            touch_input: String::new(),
            show_line_numbers: false,
            preview_wrap: false,
            show_dir_counts: true,
            show_timestamps: false,
            symlink_mode: false,
            symlink_input: String::new(),
            symlink_target: None,
            mark_set_mode: false,
            mark_jump_mode: false,
            marks: HashMap::new(),
            yank_picker_mode: false,
            context_bundle_picker_mode: false,
            context_bundle_confirm_mode: false,
            context_bundle_pending: None,
            dup_mode: false,
            dup_input: String::new(),
            dup_src: None,
            clipboard_inspect_mode: false,
            frecency_mode: false,
            frecency_list: Vec::new(),
            frecency_filtered: Vec::new(),
            frecency_selected: 0,
            frecency_query: String::new(),
            last_click_time: None,
            last_click_pos: None,
            preview_loading: false,
            preview_rx: None,
            recursive_watcher,
            change_feed: change_feed::ChangeFeed::new(),
            change_feed_mode: false,
            change_feed_root: feed_root,
            task_manager: task_manager::TaskManager::new(),
            task_manager_mode: false,
            task_pending: Vec::new(),
            archive_mode: false,
            archive_path: None,
            archive_virt_dir: String::new(),
            archive_flat_paths: Vec::new(),
            session_snapshot: None,
            session_summary_mode: false,
            session_summary_cache: None,
            session_summary_total: 0,
            session_summary_selected: 0,
        };
        app.load_dir();
        Ok(app)
    }

    /// Reload the current directory listing and parent listing.
    ///
    /// Errors are surfaced via `status_message` rather than propagated, so the
    /// app stays alive even if the working directory becomes unreadable.
    pub fn load_dir(&mut self) {
        match Self::read_entries(&self.cwd, self.show_hidden, self.sort_mode, self.sort_order) {
            Ok((entries, truncated)) => {
                self.entries = entries;
                self.entries_truncated = truncated;
            }
            Err(e) => {
                self.entries = Vec::new();
                self.entries_truncated = false;
                self.status_message = Some(format!("Cannot read directory: {e}"));
            }
        }

        // Re-apply active filter to the freshly loaded entries.
        if !self.filter_input.is_empty() {
            let pattern = self.filter_input.to_lowercase();
            self.entries
                .retain(|e| e.name.to_lowercase().contains(&pattern));
        }

        // Hide gitignored entries when the toggle is active.
        if self.hide_gitignored {
            self.gitignored_names = crate::git::load_ignored(&self.cwd);
            if !self.gitignored_names.is_empty() {
                self.entries
                    .retain(|e| !self.gitignored_names.contains(&e.name));
            }
        } else {
            self.gitignored_names.clear();
        }

        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }

        // Parent entries (errors here are non-fatal; left pane simply stays empty).
        if let Some(parent) = self.cwd.parent() {
            match Self::read_entries(parent, self.show_hidden, self.sort_mode, self.sort_order) {
                Ok((entries, _)) => {
                    self.parent_selected =
                        entries.iter().position(|e| e.path == self.cwd).unwrap_or(0);
                    self.parent_entries = entries;
                }
                Err(_) => {
                    self.parent_entries.clear();
                    self.parent_selected = 0;
                }
            }
        } else {
            self.parent_entries.clear();
            self.parent_selected = 0;
        }

        // Refresh git status whenever we navigate to a new directory.
        self.git_status = GitStatus::load(&self.cwd);
        self.diff_preview_mode = false;

        // Replace the watcher so it tracks the current directory.
        // Only recreate it when watching is active (watcher is Some).
        if self.watcher.is_some() {
            self.watcher = crate::watcher::DirWatcher::new(&self.cwd);
        }

        self.load_preview();
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
    pub fn sort_entries(entries: &mut [DirEntry], mode: SortMode, order: SortOrder) {
        entries.sort_by(|a, b| {
            // Directories always before files regardless of sort mode.
            let dir_cmp = b.is_dir.cmp(&a.is_dir);
            if dir_cmp != std::cmp::Ordering::Equal {
                return dir_cmp;
            }

            let ord = match mode {
                SortMode::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortMode::Size => a.size.cmp(&b.size),
                SortMode::Modified => a.modified.cmp(&b.modified),
                SortMode::Extension => {
                    let ext = |e: &DirEntry| {
                        Path::new(&e.name)
                            .extension()
                            .and_then(|x| x.to_str())
                            .unwrap_or("")
                            .to_lowercase()
                    };
                    ext(a)
                        .cmp(&ext(b))
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
            };

            if order == SortOrder::Descending {
                ord.reverse()
            } else {
                ord
            }
        });
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
