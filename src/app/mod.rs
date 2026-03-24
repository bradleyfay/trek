use crate::git::GitStatus;
use crate::highlight::Highlighter;
use crate::ops::Clipboard;
use crate::rename::{RenameField, RenamePreviewRow};
use crate::search::SearchResultGroup;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

mod bookmarks;
mod content;
mod file_ops;
mod filter;
mod find;
mod gitignore;
mod layout;
mod metadata;
mod mouse;
mod navigation;
pub mod palette;
mod palette_ops;
mod preview;
mod quick_rename;
mod rename;
mod search;
mod sort;
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

    // --- Bulk rename ---
    /// True while the rename input bar is open.
    pub rename_mode: bool,
    /// Indices into `entries` that the user has marked for renaming.
    pub rename_selected: HashSet<usize>,
    /// Regex pattern typed by the user.
    pub rename_pattern: String,
    /// Replacement template typed by the user.
    pub rename_replacement: String,
    /// Which rename field currently has keyboard focus.
    pub rename_focus: RenameField,
    /// Live preview rows recomputed on every keystroke.
    pub rename_preview: Vec<RenamePreviewRow>,
    /// Set when the pattern is an invalid regex.
    pub rename_error: Option<String>,

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
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: std::time::SystemTime,
}

impl App {
    pub fn new(start_dir: Option<PathBuf>) -> Result<Self> {
        let cwd = match start_dir {
            Some(dir) => dir,
            None => std::env::current_dir()?,
        };
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
            chmod_mode: false,
            chmod_input: String::new(),
            highlighter: Highlighter::new(),
            clipboard: None,
            pending_delete: Vec::new(),
            last_trashed: Vec::new(),
            mkdir_mode: false,
            mkdir_input: String::new(),
            content_search_mode: false,
            content_search_query: String::new(),
            content_search_results: Vec::new(),
            content_search_selected: 0,
            content_search_error: None,
            content_search_truncated: false,
            rename_mode: false,
            rename_selected: HashSet::new(),
            rename_pattern: String::new(),
            rename_replacement: String::new(),
            rename_focus: RenameField::Pattern,
            rename_preview: Vec::new(),
            rename_error: None,
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
                Some(DirEntry {
                    name,
                    path: e.path(),
                    is_dir,
                    size,
                    modified,
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
    let ss = secs % 60;
    let mm = (secs / 60) % 60;
    let hh = (secs / 3_600) % 24;
    let mut days = secs / 86_400;

    let mut year = 1970u32;
    loop {
        let dy = if is_leap_year(year) { 366u64 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }

    let month_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hh, mm, ss
    )
}

fn is_leap_year(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
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
