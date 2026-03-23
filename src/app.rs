use crate::icons::icon_for_entry;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Index of the selected entry.
    pub selected: usize,
    /// Entries in the parent directory (for left pane).
    pub parent_entries: Vec<DirEntry>,
    /// Index of cwd within its parent listing.
    pub parent_selected: usize,
    /// Lines of the previewed file (right pane).
    pub preview_lines: Vec<String>,
    /// Preview scroll offset (line index of top visible line).
    pub preview_scroll: usize,
    /// Total height of the terminal (set during draw).
    pub term_height: u16,
    /// Total width of the terminal (set during draw).
    pub term_width: u16,

    // --- Pane layout (percentage-based, 0.0..1.0) ---
    /// Fraction of width where the left divider sits.
    pub left_div: f64,
    /// Fraction of width where the right divider sits.
    pub right_div: f64,

    // --- Drag state ---
    pub drag: Option<DragTarget>,

    // --- Pixel positions of dividers (set during draw) ---
    pub left_div_col: u16,
    pub right_div_col: u16,

    /// Area of the preview pane (set during draw).
    pub preview_area: (u16, u16, u16, u16), // (x, y, width, height)
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let mut app = Self {
            cwd: cwd.clone(),
            entries: Vec::new(),
            selected: 0,
            parent_entries: Vec::new(),
            parent_selected: 0,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            term_height: 0,
            term_width: 0,
            left_div: 0.20,
            right_div: 0.55,
            drag: None,
            left_div_col: 0,
            right_div_col: 0,
            preview_area: (0, 0, 0, 0),
        };
        app.load_dir()?;
        Ok(app)
    }

    /// Reload the current directory listing and parent listing.
    pub fn load_dir(&mut self) -> Result<()> {
        self.entries = Self::read_entries(&self.cwd);
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        // Parent entries
        if let Some(parent) = self.cwd.parent() {
            self.parent_entries = Self::read_entries(parent);
            self.parent_selected = self
                .parent_entries
                .iter()
                .position(|e| e.path == self.cwd)
                .unwrap_or(0);
        } else {
            self.parent_entries.clear();
            self.parent_selected = 0;
        }
        self.load_preview();
        Ok(())
    }

    fn read_entries(dir: &Path) -> Vec<DirEntry> {
        let mut entries: Vec<DirEntry> = match fs::read_dir(dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| {
                    let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    DirEntry {
                        name: e.file_name().to_string_lossy().into_owned(),
                        path: e.path(),
                        is_dir,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        // Sort: directories first, then alphabetical (case-insensitive).
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        entries
    }

    pub fn load_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview_lines.clear();

        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                // Show directory contents as preview.
                let children = Self::read_entries(&entry.path);
                self.preview_lines = children.iter().map(|c| {
                    let icon = icon_for_entry(&c.name, c.is_dir);
                    format!("{} {}", icon, c.name)
                }).collect();
            } else {
                // Try to read as text.
                self.preview_lines = Self::read_file_preview(&entry.path);
            }
        }
    }

    fn read_file_preview(path: &PathBuf) -> Vec<String> {
        // Read up to 2000 lines / 512KB.
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => return vec!["[cannot read file]".to_string()],
        };
        if data.len() > 512 * 1024 {
            return vec!["[file too large to preview]".to_string()];
        }
        // Check for binary content (null bytes in first 8KB).
        let check_len = data.len().min(8192);
        if data[..check_len].contains(&0) {
            return vec!["[binary file]".to_string()];
        }
        let text = String::from_utf8_lossy(&data);
        text.lines()
            .take(2000)
            .map(|l| l.to_string())
            .collect()
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
            let old_name = self.cwd.file_name().map(|n| n.to_string_lossy().into_owned());
            self.cwd = parent;
            let _ = self.load_dir();
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
                let _ = self.load_dir();
            }
        }
    }

    pub fn go_home(&mut self) {
        if let Some(home) = dirs_home() {
            self.cwd = home;
            self.selected = 0;
            let _ = self.load_dir();
        }
    }

    // --- Mouse handling ---

    /// Called on left-button mouse down. Check if the click is on a divider.
    pub fn on_mouse_down(&mut self, col: u16, _row: u16) {
        const GRAB_MARGIN: u16 = 1;
        if col.abs_diff(self.left_div_col) <= GRAB_MARGIN {
            self.drag = Some(DragTarget::LeftDivider);
        } else if col.abs_diff(self.right_div_col) <= GRAB_MARGIN {
            self.drag = Some(DragTarget::RightDivider);
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

    /// Scroll up in the preview pane (if cursor is over it).
    pub fn on_scroll_up(&mut self, col: u16, row: u16) {
        if self.is_in_preview(col, row) {
            self.preview_scroll = self.preview_scroll.saturating_sub(3);
        }
    }

    /// Scroll down in the preview pane (if cursor is over it).
    pub fn on_scroll_down(&mut self, col: u16, row: u16) {
        if self.is_in_preview(col, row) {
            let max_scroll = self.preview_lines.len().saturating_sub(1);
            self.preview_scroll = (self.preview_scroll + 3).min(max_scroll);
        }
    }

    fn is_in_preview(&self, col: u16, row: u16) -> bool {
        let (x, y, w, h) = self.preview_area;
        col >= x && col < x + w && row >= y && row < y + h
    }
}

/// Get the user's home directory without pulling in another crate.
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
