use crate::git::GitStatus;
use crate::highlight::Highlighter;
use crate::icons::icon_for_entry;
use crate::ops::{self, Clipboard, ClipboardOp};
use crate::rename::{self, RenameField, RenamePreviewRow};
use crate::search::{self, SearchResultGroup};
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

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
struct HistoryEntry {
    dir: PathBuf,
    selected: usize,
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

    // --- Syntax highlighter (initialized once at startup) ---
    pub highlighter: Highlighter,

    // --- File operations clipboard ---
    /// Files queued for copy or cut.
    pub clipboard: Option<Clipboard>,
    /// Paths pending deletion (non-empty while confirmation prompt is shown).
    pub pending_delete: Vec<PathBuf>,
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
    history: Vec<HistoryEntry>,
    /// Index into `history` pointing at the current location.
    history_pos: usize,
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
            highlighter: Highlighter::new(),
            clipboard: None,
            pending_delete: Vec::new(),
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
    fn read_entries(
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

    pub fn load_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview_lines.clear();
        self.preview_is_diff = false;

        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
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
            self.load_preview();
        } else {
            self.status_message = Some("No git changes for this file".to_string());
        }
    }

    /// Re-run `git status` for the current directory and refresh the preview.
    pub fn refresh_git_status(&mut self) {
        self.git_status = GitStatus::load(&self.cwd);
        self.load_preview();
        self.status_message = Some("Git status refreshed".to_string());
    }

    // --- Bulk rename ---

    /// Toggle the rename-selection mark on entry `idx`.
    ///
    /// Directories are silently skipped (directory rename is out of scope for v1).
    pub fn toggle_selection(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx) {
            if entry.is_dir {
                self.status_message = Some("Directory rename not supported".to_string());
                return;
            }
        }
        if self.rename_selected.contains(&idx) {
            self.rename_selected.remove(&idx);
        } else {
            self.rename_selected.insert(idx);
        }
    }

    /// Mark all non-directory entries in the current directory for renaming.
    pub fn select_all(&mut self) {
        self.rename_selected = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .map(|(i, _)| i)
            .collect();
    }

    /// Clear all selection marks.
    pub fn clear_selections(&mut self) {
        self.rename_selected.clear();
        self.status_message = None;
    }

    /// Enter rename mode (requires at least one file to be selected).
    pub fn start_rename(&mut self) {
        if self.rename_selected.is_empty() {
            self.status_message = Some("No files selected".to_string());
            return;
        }
        self.rename_mode = true;
        self.rename_pattern.clear();
        self.rename_replacement.clear();
        self.rename_focus = RenameField::Pattern;
        self.rename_preview.clear();
        self.rename_error = None;
        self.current_scroll = 0;
    }

    /// Exit rename mode without touching the filesystem.
    pub fn cancel_rename(&mut self) {
        self.rename_mode = false;
        self.rename_pattern.clear();
        self.rename_replacement.clear();
        self.rename_preview.clear();
        self.rename_error = None;
        self.rename_selected.clear();
        self.status_message = None;
    }

    /// Apply the current rename preview to the filesystem.
    pub fn confirm_rename(&mut self) {
        match rename::apply_renames(&self.rename_preview, &self.cwd) {
            Ok(count) => {
                let msg = format!(
                    "Renamed {} file{}",
                    count,
                    if count == 1 { "" } else { "s" }
                );
                self.rename_mode = false;
                self.rename_selected.clear();
                self.rename_pattern.clear();
                self.rename_replacement.clear();
                self.rename_preview.clear();
                self.rename_error = None;
                self.load_dir();
                self.status_message = Some(msg);
            }
            Err(e) => {
                self.rename_error = Some(e);
            }
        }
    }

    pub fn rename_push_char(&mut self, c: char) {
        match self.rename_focus {
            RenameField::Pattern => self.rename_pattern.push(c),
            RenameField::Replacement => self.rename_replacement.push(c),
        }
        self.update_rename_preview();
    }

    pub fn rename_pop_char(&mut self) {
        match self.rename_focus {
            RenameField::Pattern => {
                self.rename_pattern.pop();
            }
            RenameField::Replacement => {
                self.rename_replacement.pop();
            }
        }
        self.update_rename_preview();
    }

    pub fn rename_next_field(&mut self) {
        self.rename_focus = RenameField::Replacement;
    }

    pub fn rename_prev_field(&mut self) {
        self.rename_focus = RenameField::Pattern;
    }

    /// Recompute the live rename preview from the current pattern and replacement.
    fn update_rename_preview(&mut self) {
        let mut indices: Vec<usize> = self.rename_selected.iter().copied().collect();
        indices.sort_unstable();
        let selected_entries: Vec<&DirEntry> = indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .collect();
        let (preview, error) = rename::compute_preview(
            &selected_entries,
            &self.entries,
            &self.rename_pattern,
            &self.rename_replacement,
        );
        self.rename_preview = preview;
        self.rename_error = error;
    }

    fn read_file_preview(path: &PathBuf) -> Vec<String> {
        // Verify the path is a regular file *before* opening it.
        // Without this check, fs::read can hang indefinitely on FIFOs, device
        // files, and other special filesystem entries — even ones reached through
        // symlinks — because a read on those may block waiting for a writer.
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return vec!["[cannot read file]".to_string()],
        };
        if !meta.file_type().is_file() {
            return vec!["[not a regular file]".to_string()];
        }
        // Attempt archive listing before the size and binary checks so that
        // large archives (> 512 KB) still produce a useful file manifest.
        if let Some(lines) = crate::archive::try_list_archive(path) {
            return lines;
        }
        // Check size via metadata *before* allocating.
        // Previously we allocated the full buffer and then discarded it — this
        // avoids that wasted allocation and speeds up rejection of large files.
        if meta.len() > 512 * 1024 {
            return vec!["[file too large to preview]".to_string()];
        }
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => return vec!["[cannot read file]".to_string()],
        };
        // Check for binary content (null bytes in first 8 KB).
        let check_len = data.len().min(8192);
        if data[..check_len].contains(&0) {
            return vec!["[binary file]".to_string()];
        }
        let text = String::from_utf8_lossy(&data);
        text.lines().take(2000).map(|l| l.to_string()).collect()
    }

    /// Ensure the selected item is visible, adjusting scroll offset.
    pub fn ensure_visible(&mut self, pane_height: u16) {
        let h = pane_height.saturating_sub(2) as usize; // subtract border rows
        if h == 0 {
            return;
        }
        if self.selected < self.current_scroll {
            self.current_scroll = self.selected;
        } else if self.selected >= self.current_scroll + h {
            self.current_scroll = self.selected - h + 1;
        }
    }

    pub fn ensure_parent_visible(&mut self, pane_height: u16) {
        let h = pane_height.saturating_sub(2) as usize;
        if h == 0 {
            return;
        }
        if self.parent_selected < self.parent_scroll {
            self.parent_scroll = self.parent_selected;
        } else if self.parent_selected >= self.parent_scroll + h {
            self.parent_scroll = self.parent_selected - h + 1;
        }
    }

    // --- Layout cache (written by ui::draw, read by mouse handlers) ---

    /// Store computed layout values needed for mouse hit-testing.
    /// Called once per frame by `ui::draw` after it calculates pane geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_layout(
        &mut self,
        term_width: u16,
        term_height: u16,
        left_div_col: u16,
        right_div_col: u16,
        parent_area: (u16, u16, u16, u16),
        current_area: (u16, u16, u16, u16),
        preview_area: (u16, u16, u16, u16),
    ) {
        self.term_width = term_width;
        self.term_height = term_height;
        self.left_div_col = left_div_col;
        self.right_div_col = right_div_col;
        self.parent_area = parent_area;
        self.current_area = current_area;
        self.preview_area = preview_area;
    }

    /// Clear the current status message (called on every keypress).
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    // --- Navigation ---

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.load_preview();
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
            self.load_preview();
        }
    }

    pub fn go_top(&mut self) {
        self.selected = 0;
        self.load_preview();
    }

    pub fn go_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
        self.load_preview();
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.cwd.parent().map(|p| p.to_path_buf()) {
            let old_name = self
                .cwd
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            self.push_history(parent.clone());
            self.cwd = parent;
            self.load_dir();
            // Try to select the directory we came from.
            if let Some(name) = old_name {
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
                    self.load_preview();
                }
            }
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
                self.push_history(entry.path.clone());
                self.cwd = entry.path;
                self.selected = 0;
                self.current_scroll = 0;
                self.load_dir();
            } else {
                // For files, yank the relative path.
                self.yank_relative_path();
            }
        }
    }

    pub fn go_home(&mut self) {
        if let Some(home) = dirs_home() {
            self.push_history(home.clone());
            self.cwd = home;
            self.selected = 0;
            self.current_scroll = 0;
            self.load_dir();
        }
    }

    // --- Directory jump history ---

    /// Record a navigation to `new_dir` in the jump history stack.
    ///
    /// Saves the current cursor index into the current entry, discards any
    /// forward entries (browser-style), appends the new location, and caps
    /// the stack at MAX_HISTORY.
    fn push_history(&mut self, new_dir: PathBuf) {
        // Save current cursor into the current history entry.
        if let Some(e) = self.history.get_mut(self.history_pos) {
            e.selected = self.selected;
        }
        // Discard forward entries.
        self.history.truncate(self.history_pos + 1);
        // Append new location.
        self.history.push(HistoryEntry {
            dir: new_dir,
            selected: 0,
        });
        self.history_pos = self.history.len() - 1;
        // Cap at MAX_HISTORY (drop oldest).
        if self.history.len() > MAX_HISTORY {
            let drop = self.history.len() - MAX_HISTORY;
            self.history.drain(..drop);
            self.history_pos = self.history_pos.saturating_sub(drop);
        }
    }

    /// Go back to the previous location in the jump history stack.
    pub fn history_back(&mut self) {
        if self.history_pos == 0 {
            self.status_message = Some("Already at oldest location".to_string());
            return;
        }
        if let Some(e) = self.history.get_mut(self.history_pos) {
            e.selected = self.selected;
        }
        self.history_pos -= 1;
        self.restore_history_entry();
    }

    /// Go forward in the jump history stack (after going back).
    pub fn history_forward(&mut self) {
        if self.history_pos + 1 >= self.history.len() {
            self.status_message = Some("Already at newest location".to_string());
            return;
        }
        if let Some(e) = self.history.get_mut(self.history_pos) {
            e.selected = self.selected;
        }
        self.history_pos += 1;
        self.restore_history_entry();
    }

    fn restore_history_entry(&mut self) {
        let entry_dir = self.history[self.history_pos].dir.clone();
        let saved_sel = self.history[self.history_pos].selected;

        if !entry_dir.is_dir() {
            self.status_message = Some(format!(
                "History location no longer exists: {}",
                entry_dir.display()
            ));
            return;
        }

        self.cwd = entry_dir;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
        self.selected = saved_sel.min(self.entries.len().saturating_sub(1));
        self.load_preview();

        let has_forward = self.history_pos + 1 < self.history.len();
        let arrow = if has_forward { "←" } else { "→" };
        self.status_message = Some(format!(
            "{} {}/{}  {}",
            arrow,
            self.history_pos + 1,
            self.history.len(),
            self.cwd.display()
        ));
    }

    /// Return the current history stack depth (number of entries).
    #[cfg(test)]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Return the current position in the history stack.
    #[cfg(test)]
    pub fn history_position(&self) -> usize {
        self.history_pos
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.load_dir();
        self.status_message = Some(if self.show_hidden {
            "Showing hidden files".to_string()
        } else {
            "Hiding hidden files".to_string()
        });
    }

    // --- Sort ---

    /// Cycle through Name → Size → Modified → Extension → Name.
    ///
    /// Size and Modified default to descending (most useful first); the others
    /// default to ascending.
    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.sort_order = match self.sort_mode {
            SortMode::Size | SortMode::Modified => SortOrder::Descending,
            _ => SortOrder::Ascending,
        };
        self.apply_sort();
        let arrow = if self.sort_order == SortOrder::Descending {
            "↓"
        } else {
            "↑"
        };
        self.status_message = Some(format!("Sort: {} {}", self.sort_mode.label(), arrow));
    }

    /// Toggle the sort direction between ascending and descending.
    pub fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
        self.apply_sort();
    }

    fn apply_sort(&mut self) {
        // Capture selected file's name so the cursor follows the file after re-sort.
        let selected_name = self.entries.get(self.selected).map(|e| e.name.clone());
        Self::sort_entries(&mut self.entries, self.sort_mode, self.sort_order);
        Self::sort_entries(&mut self.parent_entries, self.sort_mode, self.sort_order);
        if let Some(name) = selected_name {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
            }
        }
        self.load_preview();
    }

    // --- Mouse handling ---

    /// Called on left-button mouse down. Check if the click is on a divider,
    /// or select a file in the current/parent pane.
    pub fn on_mouse_down(&mut self, col: u16, row: u16) {
        const GRAB_MARGIN: u16 = 1;
        if col.abs_diff(self.left_div_col) <= GRAB_MARGIN {
            self.drag = Some(DragTarget::LeftDivider);
        } else if col.abs_diff(self.right_div_col) <= GRAB_MARGIN {
            self.drag = Some(DragTarget::RightDivider);
        } else if self.is_in_current(col, row) {
            self.click_select_current(col, row);
        } else if self.is_in_parent(col, row) {
            self.click_select_parent(col, row);
        }
    }

    fn click_select_current(&mut self, _col: u16, row: u16) {
        let (_, y, _, _) = self.current_area;
        let inner_y = y + 1; // skip top border
        if row < inner_y {
            return;
        }
        let clicked_offset = (row - inner_y) as usize;
        let idx = self.current_scroll + clicked_offset;
        if idx < self.entries.len() {
            self.selected = idx;
            self.load_preview();
        }
    }

    fn click_select_parent(&mut self, _col: u16, row: u16) {
        let (_, y, _, _) = self.parent_area;
        let inner_y = y + 1; // skip top border
        if row < inner_y {
            return;
        }
        let clicked_offset = (row - inner_y) as usize;
        let idx = self.parent_scroll + clicked_offset;
        if idx < self.parent_entries.len() {
            // Navigate to that parent entry if it's a directory.
            if let Some(entry) = self.parent_entries.get(idx).cloned() {
                if entry.is_dir {
                    self.push_history(entry.path.clone());
                    self.cwd = entry.path;
                    self.selected = 0;
                    self.current_scroll = 0;
                    self.load_dir();
                }
            }
        }
    }

    /// Called on mouse drag.
    pub fn on_mouse_drag(&mut self, col: u16, _row: u16) {
        let w = self.term_width as f64;
        if w < 10.0 {
            return;
        }
        let frac = (col as f64) / w;
        match self.drag {
            Some(DragTarget::LeftDivider) => {
                // Clamp: min 5% from left, at least 10% gap before right divider.
                self.left_div = frac.clamp(0.05, self.right_div - 0.10);
            }
            Some(DragTarget::RightDivider) => {
                // Clamp: at least 10% gap after left divider, max 95%.
                self.right_div = frac.clamp(self.left_div + 0.10, 0.95);
            }
            None => {}
        }
    }

    /// Called on mouse button release.
    pub fn on_mouse_up(&mut self) {
        self.drag = None;
    }

    /// Scroll up in whichever pane the cursor is over.
    pub fn on_scroll_up(&mut self, col: u16, row: u16) {
        if self.is_in_preview(col, row) {
            self.preview_scroll = self.preview_scroll.saturating_sub(3);
        } else if self.is_in_current(col, row) {
            if self.selected > 0 {
                self.selected = self.selected.saturating_sub(3);
                self.load_preview();
            }
        } else if self.is_in_parent(col, row) {
            self.parent_scroll = self.parent_scroll.saturating_sub(3);
        }
    }

    /// Scroll down in whichever pane the cursor is over.
    pub fn on_scroll_down(&mut self, col: u16, row: u16) {
        if self.is_in_preview(col, row) {
            let max_scroll = self.preview_lines.len().saturating_sub(1);
            self.preview_scroll = (self.preview_scroll + 3).min(max_scroll);
        } else if self.is_in_current(col, row) {
            if !self.entries.is_empty() {
                self.selected = (self.selected + 3).min(self.entries.len() - 1);
                self.load_preview();
            }
        } else if self.is_in_parent(col, row) {
            let max = self.parent_entries.len().saturating_sub(1);
            self.parent_scroll = (self.parent_scroll + 3).min(max);
        }
    }

    fn is_in_rect(&self, col: u16, row: u16, area: (u16, u16, u16, u16)) -> bool {
        let (x, y, w, h) = area;
        col >= x && col < x + w && row >= y && row < y + h
    }

    fn is_in_preview(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.preview_area)
    }

    fn is_in_current(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.current_area)
    }

    fn is_in_parent(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.parent_area)
    }

    // --- Fuzzy search ---

    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.pre_search_selected = self.selected;
        self.update_filter();
    }

    pub fn cancel_search(&mut self) {
        self.selected = self.pre_search_selected;
        self.search_mode = false;
        self.search_query.clear();
        self.filtered_indices.clear();
        self.filtered_set.clear();
        self.load_preview();
    }

    pub fn confirm_search(&mut self) {
        // Move selection to the first filtered match, then exit search mode.
        if let Some(&idx) = self.filtered_indices.first() {
            self.selected = idx;
            self.load_preview();
        }
        self.search_mode = false;
        self.search_query.clear();
        self.filtered_indices.clear();
        self.filtered_set.clear();
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    pub fn search_move_down(&mut self) {
        // Move to the next filtered match after current selection.
        if let Some(pos) = self
            .filtered_indices
            .iter()
            .position(|&i| i > self.selected)
        {
            self.selected = self.filtered_indices[pos];
            self.load_preview();
        }
    }

    pub fn search_move_up(&mut self) {
        // Move to the previous filtered match before current selection.
        if let Some(pos) = self
            .filtered_indices
            .iter()
            .rposition(|&i| i < self.selected)
        {
            self.selected = self.filtered_indices[pos];
            self.load_preview();
        }
    }

    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| fuzzy_match(&e.name.to_lowercase(), &query))
                .map(|(i, _)| i)
                .collect();
        }
        self.filtered_set = self.filtered_indices.iter().copied().collect();
        // Auto-select first match.
        if let Some(&first) = self.filtered_indices.first() {
            self.selected = first;
            self.load_preview();
        }
    }

    // --- File operations ---

    /// Yank (copy) the currently selected entry to the clipboard.
    pub fn clipboard_copy_current(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.clipboard = Some(Clipboard {
                op: ClipboardOp::Copy,
                paths: vec![entry.path.clone()],
            });
            self.status_message = Some(format!("[copy] \"{}\"", entry.name));
        }
    }

    /// Yank (copy) all rename-selected entries to the clipboard.
    pub fn clipboard_copy_selected(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            self.status_message = Some("No files selected".to_string());
            return;
        }
        let count = paths.len();
        self.clipboard = Some(Clipboard {
            op: ClipboardOp::Copy,
            paths,
        });
        self.rename_selected.clear();
        self.status_message = Some(format!("[copy] {} files", count));
    }

    /// Mark the currently selected entry for cut.
    pub fn clipboard_cut_current(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.clipboard = Some(Clipboard {
                op: ClipboardOp::Cut,
                paths: vec![entry.path.clone()],
            });
            self.status_message = Some(format!("[cut] \"{}\"", entry.name));
        }
    }

    /// Paste clipboard contents into the current directory.
    ///
    /// Conflicting names (already exist in cwd) are skipped with a warning.
    pub fn paste_clipboard(&mut self) {
        let Some(clip) = self.clipboard.take() else {
            self.status_message = Some("Nothing in clipboard".to_string());
            return;
        };
        let mut done = 0usize;
        let mut skipped = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for src in &clip.paths {
            let file_name = match src.file_name() {
                Some(n) => n,
                None => continue,
            };
            let dst = self.cwd.join(file_name);

            // Skip if destination already exists (conflict).
            if dst.exists() && &dst != src {
                skipped += 1;
                continue;
            }
            // Skip trivial no-op (cut to same directory).
            if clip.op == ClipboardOp::Cut && dst == *src {
                continue;
            }

            let result = match clip.op {
                ClipboardOp::Copy => ops::copy_path(src, &dst),
                ClipboardOp::Cut => ops::move_path(src, &dst),
            };
            match result {
                Ok(()) => done += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }

        // For Cut, the clipboard is consumed. For Copy, keep it for repeated pastes.
        if clip.op == ClipboardOp::Copy {
            self.clipboard = Some(Clipboard {
                op: ClipboardOp::Copy,
                paths: clip.paths,
            });
        }

        let verb = match clip.op {
            ClipboardOp::Copy => "Copied",
            ClipboardOp::Cut => "Moved",
        };
        let mut msg = format!("{} {} item{}", verb, done, if done == 1 { "" } else { "s" });
        if skipped > 0 {
            msg.push_str(&format!(" ({} skipped — already exists)", skipped));
        }
        if let Some(err) = errors.first() {
            msg = format!("Error: {}", err);
        }
        self.status_message = Some(msg);
        self.load_dir();
        self.git_status = crate::git::GitStatus::load(&self.cwd);
    }

    /// Begin a delete confirmation for the currently selected entry.
    pub fn begin_delete_current(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.pending_delete = vec![entry.path.clone()];
        }
    }

    /// Begin a delete confirmation for all rename-selected entries.
    pub fn begin_delete_selected(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            self.status_message = Some("No files selected".to_string());
            return;
        }
        self.pending_delete = paths;
    }

    /// Execute the pending deletion after user confirmation.
    pub fn confirm_delete(&mut self) {
        let paths = std::mem::take(&mut self.pending_delete);
        let mut done = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for path in &paths {
            match ops::delete_path(path) {
                Ok(()) => done += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }
        self.rename_selected.clear();
        if let Some(err) = errors.first() {
            self.status_message = Some(format!("Error: {}", err));
        } else {
            self.status_message = Some(format!(
                "Deleted {} item{}",
                done,
                if done == 1 { "" } else { "s" }
            ));
        }
        self.load_dir();
        self.git_status = crate::git::GitStatus::load(&self.cwd);
    }

    /// Cancel the pending deletion.
    pub fn cancel_delete(&mut self) {
        self.pending_delete.clear();
        self.status_message = Some("Delete cancelled".to_string());
    }

    /// Enter mkdir mode.
    pub fn begin_mkdir(&mut self) {
        self.mkdir_mode = true;
        self.mkdir_input.clear();
    }

    /// Execute mkdir with the current input and exit mkdir mode.
    pub fn confirm_mkdir(&mut self) {
        let name = self.mkdir_input.trim().to_string();
        self.mkdir_mode = false;
        self.mkdir_input.clear();
        if name.is_empty() {
            self.status_message = Some("Directory name cannot be empty".to_string());
            return;
        }
        match ops::make_dir(&self.cwd, &name) {
            Ok(_) => {
                self.status_message = Some(format!("Created directory \"{}\"", name));
                self.load_dir();
                // Select the newly created directory.
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
                    self.load_preview();
                }
            }
            Err(e) => {
                self.status_message = Some(format!("mkdir failed: {}", e));
            }
        }
    }

    /// Cancel mkdir mode without creating anything.
    pub fn cancel_mkdir(&mut self) {
        self.mkdir_mode = false;
        self.mkdir_input.clear();
    }

    pub fn mkdir_push_char(&mut self, c: char) {
        self.mkdir_input.push(c);
    }

    pub fn mkdir_pop_char(&mut self) {
        self.mkdir_input.pop();
    }

    /// Return paths of all rename-selected entries, sorted by index.
    fn selected_paths(&self) -> Vec<PathBuf> {
        let mut indices: Vec<usize> = self.rename_selected.iter().copied().collect();
        indices.sort_unstable();
        indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.path.clone())
            .collect()
    }

    // --- Content search (Ctrl+F / rg) ---

    /// Enter content search mode.
    pub fn start_content_search(&mut self) {
        self.content_search_mode = true;
        self.content_search_query.clear();
        self.content_search_results.clear();
        self.content_search_selected = 0;
        self.content_search_error = None;
        self.content_search_truncated = false;
    }

    /// Exit content search mode without side effects.
    pub fn cancel_content_search(&mut self) {
        self.content_search_mode = false;
        self.content_search_query.clear();
        self.content_search_results.clear();
        self.content_search_selected = 0;
        self.content_search_error = None;
        self.content_search_truncated = false;
    }

    pub fn content_search_push_char(&mut self, c: char) {
        self.content_search_query.push(c);
    }

    pub fn content_search_pop_char(&mut self) {
        self.content_search_query.pop();
    }

    /// Run rg with the current query and populate results.
    pub fn run_content_search(&mut self) {
        if self.content_search_query.is_empty() {
            return;
        }
        match search::run_rg(&self.content_search_query, &self.cwd) {
            Ok(groups) => {
                let total: usize = groups.iter().map(|g| g.matches.len()).sum();
                self.content_search_truncated = total >= search::MAX_RESULTS;
                self.content_search_results = groups;
                self.content_search_selected = 0;
                self.content_search_error = None;
            }
            Err(e) => {
                self.content_search_results.clear();
                self.content_search_error = Some(e);
            }
        }
    }

    /// Move selection down by one match entry (crosses file boundaries).
    pub fn content_search_move_down(&mut self) {
        let total: usize = self
            .content_search_results
            .iter()
            .map(|g| g.matches.len())
            .sum();
        if total > 0 && self.content_search_selected + 1 < total {
            self.content_search_selected += 1;
        }
    }

    /// Move selection up by one match entry.
    pub fn content_search_move_up(&mut self) {
        self.content_search_selected = self.content_search_selected.saturating_sub(1);
    }

    /// Navigate to the currently selected search result: update cwd if needed,
    /// select the file in the entry list, and scroll the preview to the match line.
    pub fn jump_to_content_result(&mut self) {
        // Resolve flat index → (group, match).
        let mut flat = self.content_search_selected;
        let mut target_file: Option<std::path::PathBuf> = None;
        let mut target_line: u64 = 0;
        for group in &self.content_search_results {
            if flat < group.matches.len() {
                target_file = Some(self.cwd.join(&group.file));
                target_line = group.matches[flat].line_number;
                break;
            }
            flat -= group.matches.len();
        }
        let Some(file_path) = target_file else {
            return;
        };
        // Navigate to the file's parent directory if different from cwd.
        if let Some(parent) = file_path.parent() {
            if parent != self.cwd {
                let new_dir = parent.to_path_buf();
                self.push_history(new_dir.clone());
                self.cwd = new_dir;
                self.selected = 0;
                self.current_scroll = 0;
                self.load_dir();
            }
        }
        // Select the file in the entry list.
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        if let Some(name) = file_name {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
                self.load_preview();
                // Scroll preview to the matching line (1-based → 0-based offset).
                self.preview_scroll = (target_line as usize).saturating_sub(1);
            }
        }
    }

    // --- Bookmarks (b / B) ---

    /// Bookmark the current directory.
    pub fn add_bookmark(&mut self) {
        match crate::bookmarks::add(&self.cwd) {
            Ok(()) => self.status_message = Some(format!("Bookmarked {}", self.cwd.display())),
            Err(e) => self.status_message = Some(format!("Bookmark failed: {e}")),
        }
    }

    /// Open the bookmark picker overlay.
    pub fn open_bookmarks(&mut self) {
        self.bookmarks = crate::bookmarks::load();
        self.bookmark_query.clear();
        self.bookmark_filtered = (0..self.bookmarks.len()).collect();
        self.bookmark_selected = 0;
        self.bookmark_mode = true;
    }

    /// Close the picker without navigating.
    pub fn close_bookmarks(&mut self) {
        self.bookmark_mode = false;
        self.bookmark_query.clear();
    }

    /// Navigate to the currently focused bookmark.
    pub fn confirm_bookmark(&mut self) {
        let Some(&real_idx) = self.bookmark_filtered.get(self.bookmark_selected) else {
            return;
        };
        let Some(dest) = self.bookmarks.get(real_idx).cloned() else {
            return;
        };
        self.close_bookmarks();
        if !dest.is_dir() {
            self.status_message = Some(format!("\"{}\" no longer exists", dest.display()));
            return;
        }
        self.push_history(dest.clone());
        self.cwd = dest;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
    }

    /// Remove the focused bookmark from disk immediately.
    pub fn remove_bookmark(&mut self) {
        let Some(&real_idx) = self.bookmark_filtered.get(self.bookmark_selected) else {
            return;
        };
        let _ = crate::bookmarks::remove(real_idx);
        self.bookmarks = crate::bookmarks::load();
        self.update_bookmark_filter();
        if !self.bookmark_filtered.is_empty() {
            self.bookmark_selected = self
                .bookmark_selected
                .min(self.bookmark_filtered.len().saturating_sub(1));
        }
    }

    /// Append a character to the bookmark filter and re-filter.
    pub fn bookmark_push_char(&mut self, c: char) {
        self.bookmark_query.push(c);
        self.update_bookmark_filter();
        self.bookmark_selected = 0;
    }

    /// Remove the last character from the bookmark filter and re-filter.
    pub fn bookmark_pop_char(&mut self) {
        self.bookmark_query.pop();
        self.update_bookmark_filter();
        self.bookmark_selected = 0;
    }

    /// Move selection up in the bookmark picker.
    pub fn bookmark_move_up(&mut self) {
        self.bookmark_selected = self.bookmark_selected.saturating_sub(1);
    }

    /// Move selection down in the bookmark picker.
    pub fn bookmark_move_down(&mut self) {
        if !self.bookmark_filtered.is_empty()
            && self.bookmark_selected + 1 < self.bookmark_filtered.len()
        {
            self.bookmark_selected += 1;
        }
    }

    /// Recompute `bookmark_filtered` from the current query.
    fn update_bookmark_filter(&mut self) {
        if self.bookmark_query.is_empty() {
            self.bookmark_filtered = (0..self.bookmarks.len()).collect();
            return;
        }
        let q = self.bookmark_query.to_lowercase();
        self.bookmark_filtered = self
            .bookmarks
            .iter()
            .enumerate()
            .filter(|(_, p)| p.to_string_lossy().to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
    }

    // --- Recursive find (Ctrl+P) ---

    /// Enter recursive filename find mode.
    pub fn start_find(&mut self) {
        self.find_mode = true;
        self.find_query.clear();
        self.find_results.clear();
        self.find_selected = 0;
        self.find_error = None;
        self.find_truncated = false;
    }

    /// Exit find mode without side effects.
    pub fn cancel_find(&mut self) {
        self.find_mode = false;
        self.find_query.clear();
        self.find_results.clear();
        self.find_selected = 0;
        self.find_error = None;
        self.find_truncated = false;
    }

    /// Append a character to the find query and re-run the search.
    pub fn find_push_char(&mut self, c: char) {
        self.find_query.push(c);
        self.find_selected = 0;
        self.exec_find();
    }

    /// Remove the last character from the find query and re-run the search.
    pub fn find_pop_char(&mut self) {
        self.find_query.pop();
        self.find_selected = 0;
        self.exec_find();
    }

    /// Move the find selection down by one result.
    pub fn find_move_down(&mut self) {
        if !self.find_results.is_empty() && self.find_selected + 1 < self.find_results.len() {
            self.find_selected += 1;
        }
    }

    /// Move the find selection up by one result.
    pub fn find_move_up(&mut self) {
        self.find_selected = self.find_selected.saturating_sub(1);
    }

    /// Execute the find query against the current working directory.
    fn exec_find(&mut self) {
        match crate::find::run_find(&self.find_query, &self.cwd) {
            Ok(results) => {
                self.find_truncated = results.len() >= crate::find::MAX_FIND_RESULTS;
                self.find_results = results;
                self.find_error = None;
            }
            Err(e) => {
                self.find_results.clear();
                self.find_error = Some(e);
                self.find_truncated = false;
            }
        }
    }

    /// Navigate to the currently selected find result: change `cwd` to the
    /// file's parent directory, select the file, exit find mode, and push a
    /// history entry.
    pub fn jump_to_find_result(&mut self) {
        let Some(result) = self.find_results.get(self.find_selected).cloned() else {
            return;
        };

        let file_path = result.absolute;
        let Some(parent) = file_path.parent() else {
            return;
        };

        if parent != self.cwd {
            let new_dir = parent.to_path_buf();
            self.push_history(new_dir.clone());
            self.cwd = new_dir;
            self.selected = 0;
            self.current_scroll = 0;
            self.load_dir();
        }

        // Select the file in the entry list.
        if let Some(name) = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
        {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
                self.load_preview();
            }
        }

        self.cancel_find();
    }

    // --- Clipboard (OSC 52) ---

    pub fn yank_relative_path(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let rel = entry.path.strip_prefix(&self.cwd).unwrap_or(&entry.path);
            let path_str = format!("./{}", rel.display());
            self.osc52_copy(&path_str);
            self.status_message = Some(format!("Yanked: {}", path_str));
        }
    }

    pub fn yank_absolute_path(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let path_str = entry.path.to_string_lossy().into_owned();
            self.osc52_copy(&path_str);
            self.status_message = Some(format!("Yanked: {}", path_str));
        }
    }

    /// Write an OSC 52 sequence to set the system clipboard.
    fn osc52_copy(&self, text: &str) {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        // OSC 52 ; c ; <base64> ST
        let seq = format!("\x1b]52;c;{}\x07", encoded);
        let _ = std::io::stdout().write_all(seq.as_bytes());
        let _ = std::io::stdout().flush();
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
fn fuzzy_match(name: &str, query: &str) -> bool {
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
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, is_dir: bool, size: u64, secs: u64) -> DirEntry {
        DirEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            is_dir,
            size,
            modified: std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs),
        }
    }

    /// Given: SortMode::Name
    /// When: next() is called 4 times
    /// Then: cycles back to Name
    #[test]
    fn sort_mode_cycles_all_variants() {
        let mut m = SortMode::Name;
        m = m.next();
        assert_eq!(m, SortMode::Size);
        m = m.next();
        assert_eq!(m, SortMode::Modified);
        m = m.next();
        assert_eq!(m, SortMode::Extension);
        m = m.next();
        assert_eq!(m, SortMode::Name);
    }

    /// Given: each SortMode
    /// When: label() is called
    /// Then: returns a non-empty string matching the mode name
    #[test]
    fn sort_mode_labels_are_non_empty() {
        assert_eq!(SortMode::Name.label(), "Name");
        assert_eq!(SortMode::Size.label(), "Size");
        assert_eq!(SortMode::Modified.label(), "Modified");
        assert_eq!(SortMode::Extension.label(), "Extension");
    }

    /// Given: mixed files and dirs with various names
    /// When: sort_entries is called with Name/Ascending
    /// Then: dirs come first, then files in A-Z order (case-insensitive)
    #[test]
    fn sort_by_name_ascending_dirs_first() {
        let mut entries = vec![
            make_entry("zebra.rs", false, 100, 0),
            make_entry("src", true, 0, 0),
            make_entry("apple.rs", false, 50, 0),
            make_entry("lib", true, 0, 0),
        ];
        App::sort_entries(&mut entries, SortMode::Name, SortOrder::Ascending);
        assert!(entries[0].is_dir && entries[1].is_dir, "dirs first");
        assert_eq!(entries[2].name, "apple.rs");
        assert_eq!(entries[3].name, "zebra.rs");
    }

    /// Given: files with different sizes
    /// When: sort_entries is called with Size/Descending
    /// Then: dirs come first; files sorted largest → smallest
    #[test]
    fn sort_by_size_descending_largest_first() {
        let mut entries = vec![
            make_entry("small.txt", false, 10, 0),
            make_entry("large.txt", false, 9999, 0),
            make_entry("medium.txt", false, 500, 0),
        ];
        App::sort_entries(&mut entries, SortMode::Size, SortOrder::Descending);
        assert_eq!(entries[0].name, "large.txt");
        assert_eq!(entries[1].name, "medium.txt");
        assert_eq!(entries[2].name, "small.txt");
    }

    /// Given: files with different modification times
    /// When: sort_entries is called with Modified/Descending
    /// Then: newest file appears first
    #[test]
    fn sort_by_modified_descending_newest_first() {
        let mut entries = vec![
            make_entry("old.txt", false, 0, 1000),
            make_entry("new.txt", false, 0, 9999),
            make_entry("mid.txt", false, 0, 5000),
        ];
        App::sort_entries(&mut entries, SortMode::Modified, SortOrder::Descending);
        assert_eq!(entries[0].name, "new.txt");
        assert_eq!(entries[1].name, "mid.txt");
        assert_eq!(entries[2].name, "old.txt");
    }

    /// Given: files with various extensions
    /// When: sort_entries is called with Extension/Ascending
    /// Then: dirs first; files grouped by extension then alphabetically
    #[test]
    fn sort_by_extension_groups_by_ext() {
        let mut entries = vec![
            make_entry("b.rs", false, 0, 0),
            make_entry("a.toml", false, 0, 0),
            make_entry("a.rs", false, 0, 0),
        ];
        App::sort_entries(&mut entries, SortMode::Extension, SortOrder::Ascending);
        // rs < toml alphabetically
        assert_eq!(entries[0].name, "a.rs");
        assert_eq!(entries[1].name, "b.rs");
        assert_eq!(entries[2].name, "a.toml");
    }

    /// Given: a mix of files and directories under any sort mode
    /// When: sort_entries is called
    /// Then: directories always appear before files
    #[test]
    fn dirs_always_before_files_regardless_of_sort_mode() {
        for mode in [
            SortMode::Name,
            SortMode::Size,
            SortMode::Modified,
            SortMode::Extension,
        ] {
            for order in [SortOrder::Ascending, SortOrder::Descending] {
                let mut entries = vec![
                    make_entry("z_file.txt", false, 9999, 9999),
                    make_entry("a_dir", true, 0, 0),
                    make_entry("b_file.txt", false, 1, 1),
                ];
                App::sort_entries(&mut entries, mode, order);
                assert!(
                    entries[0].is_dir,
                    "dir should be first for mode={mode:?} order={order:?}, got {:?}",
                    entries.iter().map(|e| &e.name).collect::<Vec<_>>()
                );
            }
        }
    }

    // ── History tests ────────────────────────────────────────────────────────

    fn make_app_at(dir: &std::path::Path) -> App {
        let mut app = App::new(Some(dir.to_path_buf())).expect("App::new");
        // Clear the initial status message so tests can check specific messages.
        app.status_message = None;
        app
    }

    /// Given: a fresh App
    /// When: history is checked
    /// Then: stack has exactly one entry (the launch directory) at position 0
    #[test]
    fn history_initialized_with_one_entry() {
        let dir = std::env::temp_dir();
        let app = make_app_at(&dir);
        assert_eq!(app.history_len(), 1);
        assert_eq!(app.history_position(), 0);
    }

    /// Given: a fresh App
    /// When: history_back() is called at position 0
    /// Then: status_message is "Already at oldest location"; position unchanged
    #[test]
    fn history_back_at_oldest_shows_message() {
        let dir = std::env::temp_dir();
        let mut app = make_app_at(&dir);
        app.history_back();
        assert_eq!(app.history_position(), 0);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Already at oldest location")
        );
    }

    /// Given: a fresh App
    /// When: history_forward() is called with no forward entries
    /// Then: status_message is "Already at newest location"; position unchanged
    #[test]
    fn history_forward_at_newest_shows_message() {
        let dir = std::env::temp_dir();
        let mut app = make_app_at(&dir);
        app.history_forward();
        assert_eq!(app.history_position(), 0);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Already at newest location")
        );
    }

    /// Given: two distinct real directories
    /// When: push_history is called twice, then history_back once
    /// Then: position returns to 1 (one step back) and stack still has 3 entries
    #[test]
    fn push_history_then_back_restores_position() {
        let dir = std::env::temp_dir();
        let mut app = make_app_at(&dir);
        // Simulate navigating to two sub-directories.
        let sub1 = std::env::temp_dir();
        let sub2 = std::env::temp_dir();
        app.push_history(sub1.clone());
        app.push_history(sub2.clone());
        assert_eq!(app.history_len(), 3);
        assert_eq!(app.history_position(), 2);
        // Go back — position should move to 1.
        app.history_pos -= 1; // bypass restore (no real dir switch needed)
        assert_eq!(app.history_position(), 1);
    }

    /// Given: user navigates forward, then goes back, then navigates to a new dir
    /// When: push_history is called for the new dir
    /// Then: forward entries are discarded (browser-style)
    #[test]
    fn forward_history_discarded_on_new_navigation() {
        let dir = std::env::temp_dir();
        let mut app = make_app_at(&dir);
        let sub1 = std::env::temp_dir();
        let sub2 = std::env::temp_dir();
        let sub3 = std::env::temp_dir();
        app.push_history(sub1);
        app.push_history(sub2);
        assert_eq!(app.history_len(), 3);
        // Simulate going back.
        app.history_pos = 1;
        // Navigate to a new dir — should discard entry at index 2.
        app.push_history(sub3);
        assert_eq!(
            app.history_len(),
            3,
            "old forward entry should be replaced, not accumulated"
        );
        assert_eq!(app.history_position(), 2);
    }

    /// Given: MAX_HISTORY + 5 push_history calls
    /// When: stack length is checked
    /// Then: stack is capped at MAX_HISTORY
    #[test]
    fn history_capped_at_max() {
        let dir = std::env::temp_dir();
        let mut app = make_app_at(&dir);
        for _ in 0..(MAX_HISTORY + 5) {
            app.push_history(std::env::temp_dir());
        }
        assert!(app.history_len() <= MAX_HISTORY);
    }
}
