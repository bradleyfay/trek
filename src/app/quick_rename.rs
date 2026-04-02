use super::App;
use std::path::PathBuf;

impl App {
    /// Open the quick rename bar pre-filled with the currently selected entry's name.
    /// No-op if there are no entries.
    pub fn begin_quick_rename(&mut self) {
        let Some(entry) = self.nav.entries.get(self.nav.selected) else {
            return;
        };
        self.overlay.quick_rename_input = entry.name.clone();
        self.overlay.quick_rename_mode = true;
    }

    /// Confirm the rename: validate, rename on disk, refresh listing, move cursor.
    pub fn confirm_quick_rename(&mut self) {
        let new_name = self.overlay.quick_rename_input.trim().to_string();

        if new_name.is_empty() {
            self.status_message = Some("Name cannot be empty".to_string());
            self.overlay.quick_rename_mode = false;
            self.overlay.quick_rename_input.clear();
            return;
        }

        let Some(entry) = self.nav.entries.get(self.nav.selected).cloned() else {
            self.overlay.quick_rename_mode = false;
            self.overlay.quick_rename_input.clear();
            return;
        };

        let old_name = entry.name.clone();

        // No-op when name unchanged.
        if new_name == old_name {
            self.overlay.quick_rename_mode = false;
            self.overlay.quick_rename_input.clear();
            return;
        }

        let new_path = entry
            .path
            .parent()
            .map(|p| p.join(&new_name))
            .unwrap_or_else(|| PathBuf::from(&new_name));

        if new_path.exists() {
            self.status_message = Some(format!("Already exists: \"{}\"", new_name));
            self.overlay.quick_rename_mode = false;
            self.overlay.quick_rename_input.clear();
            return;
        }

        self.overlay.quick_rename_mode = false;
        self.overlay.quick_rename_input.clear();

        match std::fs::rename(&entry.path, &new_path) {
            Ok(()) => {
                self.status_message = Some(format!(
                    "Renamed \"{}\" \u{2192} \"{}\"",
                    old_name, new_name
                ));
                self.load_dir();
                if let Some(idx) = self.nav.entries.iter().position(|e| e.name == new_name) {
                    self.nav.selected = idx;
                    self.load_preview();
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Rename failed: {}", e));
            }
        }
    }

    /// Cancel the quick rename bar without touching the filesystem.
    pub fn cancel_quick_rename(&mut self) {
        self.overlay.quick_rename_mode = false;
        self.overlay.quick_rename_input.clear();
    }

    /// Append a character to the rename input.
    pub fn quick_rename_push_char(&mut self, c: char) {
        self.overlay.quick_rename_input.push(c);
    }

    /// Remove the last character from the rename input.
    pub fn quick_rename_pop_char(&mut self) {
        self.overlay.quick_rename_input.pop();
    }
}
