use std::collections::HashSet;
use std::path::PathBuf;

use super::{DirEntry, HistoryEntry, SortMode, SortOrder};

pub struct NavigationState {
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
    /// Parent directory last loaded into `parent_entries`; used to skip
    /// redundant disk reads when `cwd` stays within the same parent.
    pub cached_parent_path: Option<PathBuf>,

    // --- Hidden files toggle ---
    pub show_hidden: bool,
    /// When true, directory entries show child item counts instead of block sizes.
    pub show_dir_counts: bool,
    /// When true, the listing shows last-modified dates instead of file sizes.
    pub show_timestamps: bool,

    // --- Sort ---
    /// Current sort field.
    pub sort_mode: SortMode,
    /// Current sort direction.
    pub sort_order: SortOrder,

    // --- Filter / narrow mode (|) ---
    /// True while the filter input bar is open (user is actively typing).
    pub filter_mode: bool,
    /// Active filter string. Empty = no filter. Non-empty while filter_mode is false
    /// means the filter is "frozen" (bar closed, listing still narrowed).
    pub filter_input: String,

    // --- Fuzzy search ---
    pub search_mode: bool,
    pub search_query: String,
    /// Indices into `entries` that match the current query.
    pub filtered_indices: Vec<usize>,
    /// O(1) membership check for filtered indices.
    pub filtered_set: HashSet<usize>,
    /// Selection before search started (for cancel-restore).
    pub pre_search_selected: usize,

    // --- Multi-file selection (Space / J / K / v) ---
    pub selection: HashSet<usize>,

    // --- Gitignore-aware listing (i) ---
    pub gitignored_names: std::collections::HashSet<String>,
    pub hide_gitignored: bool,

    // --- Directory jump history ---
    /// Chronological list of visited locations; `history[history_pos]` is current.
    pub(crate) history: Vec<HistoryEntry>,
    /// Index into `history` pointing at the current location.
    pub(crate) history_pos: usize,

    // --- Double-click detection ---
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_pos: Option<(u16, u16)>,

    // --- Frecency (visited-directory log, used for z overlay) ---
    /// All recorded frecency entries (unsorted, session-scoped).
    /// This is navigation data; the frecency overlay's UI state
    /// (frecency_mode, frecency_filtered, etc.) lives in OverlayState.
    pub frecency_list: Vec<crate::app::frecency::FrecencyEntry>,
}

impl NavigationState {
    pub fn new(cwd: PathBuf) -> Self {
        use super::HistoryEntry;
        let history_entry = HistoryEntry {
            dir: cwd.clone(),
            selected: 0,
        };
        Self {
            cwd,
            entries: Vec::new(),
            entries_truncated: false,
            selected: 0,
            current_scroll: 0,
            parent_entries: Vec::new(),
            parent_selected: 0,
            parent_scroll: 0,
            cached_parent_path: None,
            show_hidden: false,
            show_dir_counts: true,
            show_timestamps: false,
            sort_mode: SortMode::default(),
            sort_order: SortOrder::default(),
            filter_mode: false,
            filter_input: String::new(),
            search_mode: false,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            filtered_set: HashSet::new(),
            pre_search_selected: 0,
            selection: HashSet::new(),
            gitignored_names: std::collections::HashSet::new(),
            hide_gitignored: false,
            history: vec![history_entry],
            history_pos: 0,
            last_click_time: None,
            last_click_pos: None,
            frecency_list: Vec::new(),
        }
    }
}
