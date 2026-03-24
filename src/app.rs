use crate::git::GitStatus;
use crate::icons::icon_for_entry;
use crate::rename::{self, RenameField, RenamePreviewRow};
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
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

impl App {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let mut app = Self {
            cwd,
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
            rename_mode: false,
            rename_selected: HashSet::new(),
            rename_pattern: String::new(),
            rename_replacement: String::new(),
            rename_focus: RenameField::Pattern,
            rename_preview: Vec::new(),
            rename_error: None,
        };
        app.load_dir();
        Ok(app)
    }

    /// Reload the current directory listing and parent listing.
    ///
    /// Errors are surfaced via `status_message` rather than propagated, so the
    /// app stays alive even if the working directory becomes unreadable.
    pub fn load_dir(&mut self) {
        match Self::read_entries(&self.cwd, self.show_hidden) {
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
            match Self::read_entries(parent, self.show_hidden) {
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
    ) -> Result<(Vec<DirEntry>, bool), std::io::Error> {
        let rd = fs::read_dir(dir)?;

        let mut entries: Vec<DirEntry> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if !show_hidden && name.starts_with('.') {
                    return None;
                }
                let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                Some(DirEntry {
                    name,
                    path: e.path(),
                    is_dir,
                    size,
                })
            })
            .take(MAX_ENTRIES + 1) // +1 lets us detect truncation
            .collect();

        let truncated = entries.len() > MAX_ENTRIES;
        if truncated {
            entries.truncate(MAX_ENTRIES);
        }

        // Sort: directories first, then alphabetical (case-insensitive).
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        Ok((entries, truncated))
    }

    pub fn load_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview_lines.clear();
        self.preview_is_diff = false;

        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
                // Directories never show a diff preview.
                if let Ok((children, _)) = Self::read_entries(&entry.path, self.show_hidden) {
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
            self.cwd = home;
            self.selected = 0;
            self.current_scroll = 0;
            self.load_dir();
        }
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
