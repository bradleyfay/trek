use super::{App, DragTarget};

impl App {
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
}
