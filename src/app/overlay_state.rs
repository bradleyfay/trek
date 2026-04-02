use std::collections::HashMap;
use std::path::PathBuf;

use crate::find::FindResult;
use crate::search::SearchResultGroup;

pub struct OverlayState {
    // --- chmod editor (P) ---
    pub chmod_mode: bool,
    pub chmod_input: String,

    // --- mkdir mode ---
    pub mkdir_mode: bool,
    pub mkdir_input: String,

    // --- Touch / new file (t) ---
    pub touch_mode: bool,
    pub touch_input: String,

    // --- Path jump bar (e) ---
    pub path_mode: bool,
    pub path_input: String,

    // --- Command palette (:) ---
    pub palette_mode: bool,
    pub palette_query: String,
    pub palette_selected: usize,
    pub palette_filtered: Vec<usize>,

    // --- Quick single-file rename (n / F2) ---
    pub quick_rename_mode: bool,
    pub quick_rename_input: String,

    // --- Bookmarks (b / B) ---
    pub bookmark_mode: bool,
    pub bookmarks: Vec<PathBuf>,
    pub bookmark_selected: usize,
    pub bookmark_query: String,
    pub bookmark_filtered: Vec<usize>,

    // --- Recursive find (Ctrl+P) ---
    pub find_mode: bool,
    pub find_query: String,
    pub find_results: Vec<FindResult>,
    pub find_selected: usize,
    pub find_error: Option<String>,
    pub find_truncated: bool,

    // --- Symlink creation (L) ---
    pub symlink_mode: bool,
    pub symlink_input: String,
    pub symlink_target: Option<PathBuf>,

    // --- Per-session marks ---
    pub mark_set_mode: bool,
    pub mark_jump_mode: bool,
    pub marks: HashMap<char, PathBuf>,

    // --- Yank picker (A) ---
    pub yank_picker_mode: bool,

    // --- AI context bundle (Ctrl+B) ---
    pub context_bundle_picker_mode: bool,
    pub context_bundle_confirm_mode: bool,
    pub context_bundle_pending: Option<String>,

    // --- File duplication (W) ---
    pub dup_mode: bool,
    pub dup_input: String,
    pub dup_src: Option<PathBuf>,

    // --- Frecency jump (z) overlay UI state ---
    // The backing frecency_list lives in NavigationState (it is navigation data).
    pub frecency_mode: bool,
    pub frecency_filtered: Vec<usize>,
    pub frecency_selected: usize,
    pub frecency_query: String,

    // --- Help overlay ---
    pub show_help: bool,

    // --- Clipboard inspector (F) ---
    pub clipboard_inspect_mode: bool,

    // --- cmux surface picker (Tab in preview focus mode) ---
    pub cmux_surface_picker_mode: bool,
    pub cmux_surfaces: Vec<crate::app::cmux::CmuxSurface>,
    pub cmux_surface_selected: usize,
    pub cmux_surface_query: String,
    pub cmux_surface_filtered: Vec<usize>,

    // --- Mode flags ---
    pub change_feed_mode: bool,
    pub task_manager_mode: bool,
    pub archive_mode: bool,
    pub session_summary_mode: bool,

    // --- Content search (Ctrl+F / rg) ---
    pub content_search_mode: bool,
    pub content_search_query: String,
    pub content_search_results: Vec<SearchResultGroup>,
    pub content_search_selected: usize,
    pub content_search_error: Option<String>,
    pub content_search_truncated: bool,
}

impl OverlayState {
    pub fn new() -> Self {
        Self {
            chmod_mode: false,
            chmod_input: String::new(),
            mkdir_mode: false,
            mkdir_input: String::new(),
            touch_mode: false,
            touch_input: String::new(),
            path_mode: false,
            path_input: String::new(),
            palette_mode: false,
            palette_query: String::new(),
            palette_selected: 0,
            palette_filtered: crate::app::palette::filter_palette(""),
            quick_rename_mode: false,
            quick_rename_input: String::new(),
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
            frecency_mode: false,
            frecency_filtered: Vec::new(),
            frecency_selected: 0,
            frecency_query: String::new(),
            show_help: false,
            clipboard_inspect_mode: false,
            cmux_surface_picker_mode: false,
            cmux_surfaces: Vec::new(),
            cmux_surface_selected: 0,
            cmux_surface_query: String::new(),
            cmux_surface_filtered: Vec::new(),
            change_feed_mode: false,
            task_manager_mode: false,
            archive_mode: false,
            session_summary_mode: false,
            content_search_mode: false,
            content_search_query: String::new(),
            content_search_results: Vec::new(),
            content_search_selected: 0,
            content_search_error: None,
            content_search_truncated: false,
        }
    }
}
