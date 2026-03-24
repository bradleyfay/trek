use super::{App, DragTarget};

/// How long (ms) between two clicks to be counted as a double-click.
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 400;

/// Maximum cell distance (rows or cols) between two clicks for them to be
/// treated as targeting the same entry.
const DOUBLE_CLICK_RADIUS: u16 = 2;

/// Pure helper — decides whether click timing and position constitute a
/// double-click.  Extracted so it can be unit-tested without App.
fn is_double_click_timing(elapsed_ms: u128, col: u16, row: u16, last_pos: (u16, u16)) -> bool {
    let same_pos = col.abs_diff(last_pos.0) <= DOUBLE_CLICK_RADIUS
        && row.abs_diff(last_pos.1) <= DOUBLE_CLICK_RADIUS;
    elapsed_ms < DOUBLE_CLICK_THRESHOLD_MS && same_pos
}

impl App {
    /// Called on left-button mouse down. Detects double-clicks and handles
    /// divider grabs and pane selection.
    pub fn on_mouse_down(&mut self, col: u16, row: u16) {
        let now = std::time::Instant::now();

        // Double-click detection: second click at same position within threshold.
        if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
            let elapsed_ms = now.duration_since(last_time).as_millis();
            if is_double_click_timing(elapsed_ms, col, row, last_pos) {
                // Reset state so a triple-click doesn't re-trigger.
                self.last_click_time = None;
                self.last_click_pos = None;
                if self.is_in_current(col, row) {
                    self.click_select_current(col, row);
                    self.open_to_the_right();
                }
                return;
            }
        }

        // Record this click for the next double-click check.
        self.last_click_time = Some(now);
        self.last_click_pos = Some((col, row));

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

    /// Called on right-button mouse down.  Selects the entry under the cursor
    /// and opens it in a new cmux tab, exactly as Enter/l would.
    pub fn on_mouse_right_down(&mut self, col: u16, row: u16) {
        if self.is_in_current(col, row) {
            self.click_select_current(col, row);
            self.open_in_cmux_tab();
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Double-click timing (pure function, no I/O) ───────────────────────

    /// Given: a second click at the same position 100 ms after the first
    /// When: is_double_click_timing is called
    /// Then: returns true (within threshold, same position)
    #[test]
    fn double_click_detected_within_threshold_same_position() {
        assert!(is_double_click_timing(100, 10, 5, (10, 5)));
    }

    /// Given: a second click 500 ms after the first (past the 400 ms threshold)
    /// When: is_double_click_timing is called
    /// Then: returns false
    #[test]
    fn double_click_not_detected_past_threshold() {
        assert!(!is_double_click_timing(500, 10, 5, (10, 5)));
    }

    /// Given: a second click exactly at the threshold boundary (400 ms)
    /// When: is_double_click_timing is called
    /// Then: returns false (threshold is strict less-than)
    #[test]
    fn double_click_not_detected_at_exact_threshold() {
        assert!(!is_double_click_timing(400, 10, 5, (10, 5)));
    }

    /// Given: a second click far from the first (more than DOUBLE_CLICK_RADIUS apart)
    /// When: is_double_click_timing is called
    /// Then: returns false even though timing is within threshold
    #[test]
    fn double_click_not_detected_at_different_position() {
        assert!(!is_double_click_timing(50, 10, 5, (20, 5)));
    }

    /// Given: a second click within DOUBLE_CLICK_RADIUS cells of the first
    /// When: is_double_click_timing is called
    /// Then: returns true (close-enough position, within threshold)
    #[test]
    fn double_click_detected_within_radius_tolerance() {
        // DOUBLE_CLICK_RADIUS == 2, so ±2 on both axes is still a match.
        assert!(is_double_click_timing(100, 12, 7, (10, 5)));
    }

    // ── Right-click selection ─────────────────────────────────────────────

    /// Given: entries loaded and the current pane positioned at (x=20, y=1)
    /// When: right-click lands on row 3 (inner_y=2, offset=1 → entry index 1)
    /// Then: app.selected is updated to 1
    ///
    /// The test targets a directory entry on purpose. `open_in_cmux_tab`
    /// returns early for directories, so no subprocess is spawned.
    #[test]
    fn right_click_selects_entry_at_clicked_row() {
        // Use the project directory — it always has multiple entries.
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut app = App::new(Some(dir)).expect("app init");

        // Simulate a pane that starts at column 20, row 1 (border at y=1,
        // so inner content starts at y=2).
        app.current_area = (20, 1, 60, 40);
        app.current_scroll = 0;

        // Make sure there is at least a second entry to select.
        assert!(
            app.entries.len() >= 2,
            "project dir must have at least 2 entries"
        );

        // Entries are sorted dirs-first.  Find the first directory index (≥0)
        // and click one row below the border to target it.
        // open_in_cmux_tab no-ops on directories, so no external process fires.
        let dir_idx = app
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("project dir must contain at least one subdirectory");

        let click_row = 2 + dir_idx as u16; // inner_y=2, offset=dir_idx
        app.on_mouse_right_down(30, click_row);

        assert_eq!(
            app.selected, dir_idx,
            "right-click should select the entry at the clicked row"
        );
    }

    /// Given: a double-click on a file entry while not inside cmux
    /// When: on_mouse_down is called a second time at the same position
    /// Then: a status message is set (open_to_the_right gracefully declines outside cmux)
    #[test]
    fn double_click_triggers_open_to_the_right_outside_cmux() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut app = App::new(Some(dir)).expect("app init");

        // Locate the first non-directory entry so open_to_the_right doesn't
        // short-circuit on a directory.
        let file_idx = app
            .entries
            .iter()
            .position(|e| !e.is_dir)
            .expect("project dir must contain at least one file");

        // current_area: border at y=1, so inner content starts at y=2.
        // Row for file_idx = inner_y + file_idx - current_scroll = 2 + file_idx.
        app.current_area = (20, 1, 60, 40);
        app.current_scroll = 0;
        let file_row = 2 + file_idx as u16;
        let click_col = 30u16;

        // Prime double-click state: first click recorded just now.
        app.last_click_time = Some(std::time::Instant::now());
        app.last_click_pos = Some((click_col, file_row));

        // Second click immediately after — elapsed ~0 ms, same position → double-click.
        app.on_mouse_down(click_col, file_row);

        // When not inside cmux open_to_the_right sets a status message.
        assert!(
            app.status_message.is_some(),
            "double-click on a file outside cmux should produce a status message"
        );
    }
}
