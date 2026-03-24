use super::{dirs_home, App, HistoryEntry, MAX_HISTORY};
use std::path::PathBuf;

impl App {
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
            self.filter_input.clear();
            self.filter_mode = false;
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
                self.filter_input.clear();
                self.filter_mode = false;
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
            self.filter_input.clear();
            self.filter_mode = false;
            self.push_history(home.clone());
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

    pub fn clear_status(&mut self) {
        self.status_message = None;
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

    // --- Directory jump history ---

    /// Record a navigation to `new_dir` in the jump history stack.
    ///
    /// Saves the current cursor index into the current entry, discards any
    /// forward entries (browser-style), appends the new location, and caps
    /// the stack at MAX_HISTORY.
    pub fn push_history(&mut self, new_dir: PathBuf) {
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
        self.filter_input.clear();
        self.filter_mode = false;
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
        self.filter_input.clear();
        self.filter_mode = false;
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

    // --- Path jump bar (e) ---

    /// Open the path jump input bar with an empty input.
    pub fn begin_path_jump(&mut self) {
        self.path_mode = true;
        self.path_input.clear();
    }

    /// Close the path jump bar without navigating.
    pub fn cancel_path_jump(&mut self) {
        self.path_mode = false;
        self.path_input.clear();
    }

    /// Confirm the path typed in the jump bar and navigate to it.
    ///
    /// - Empty input → silently close the bar.
    /// - `~` prefix → expand to home directory.
    /// - Existing directory → navigate there.
    /// - Existing file → navigate to its parent and select the file.
    /// - Non-existent path → show an error and keep the bar open.
    pub fn confirm_path_jump(&mut self) {
        if self.path_input.is_empty() {
            self.path_mode = false;
            return;
        }

        let raw = self.path_input.clone();
        let expanded = if raw.starts_with('~') {
            match dirs_home() {
                Some(home) => {
                    let rest = raw.trim_start_matches('~').trim_start_matches('/');
                    if rest.is_empty() {
                        home
                    } else {
                        home.join(rest)
                    }
                }
                None => std::path::PathBuf::from(&raw),
            }
        } else {
            std::path::PathBuf::from(&raw)
        };

        if expanded.is_dir() {
            self.path_mode = false;
            self.path_input.clear();
            self.filter_input.clear();
            self.filter_mode = false;
            self.push_history(expanded.clone());
            self.cwd = expanded;
            self.selected = 0;
            self.current_scroll = 0;
            self.load_dir();
        } else if expanded.is_file() {
            if let Some(parent) = expanded.parent().map(|p| p.to_path_buf()) {
                let file_name = expanded
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned());
                self.path_mode = false;
                self.path_input.clear();
                self.filter_input.clear();
                self.filter_mode = false;
                self.push_history(parent.clone());
                self.cwd = parent;
                self.selected = 0;
                self.current_scroll = 0;
                self.load_dir();
                if let Some(name) = file_name {
                    if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                        self.selected = idx;
                        self.load_preview();
                    }
                }
            }
        } else {
            self.status_message = Some(format!("Path not found: {}", expanded.display()));
            // Keep bar open so user can correct the path.
        }
    }

    /// Append a character to the path jump input.
    pub fn path_push_char(&mut self, c: char) {
        self.path_input.push(c);
    }

    /// Remove the last character from the path jump input.
    pub fn path_pop_char(&mut self) {
        self.path_input.pop();
    }

    /// Returns the path of the currently selected file (not directory), or None.
    /// Used by the open-in-editor (`o`) handler which should not act on directories.
    pub fn selected_file_path(&self) -> Option<PathBuf> {
        self.entries
            .get(self.selected)
            .filter(|e| !e.is_dir)
            .map(|e| e.path.clone())
    }

    /// Returns the path of the currently selected entry (file or directory).
    /// Used by the open-with-system-default (`O`) handler.
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.entries.get(self.selected).map(|e| e.path.clone())
    }
}
