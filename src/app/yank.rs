use super::App;
use std::io::Write;

impl App {
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

    /// Copy just the filename (not the full path) to the clipboard.
    pub fn yank_filename(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let name = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry.name.clone());
            self.osc52_copy(&name);
            self.status_message = Some(format!("[yank] {}", name));
        }
    }

    /// Copy the parent directory of the selected entry to the clipboard.
    pub fn yank_parent_dir(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let parent = entry
                .path
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "/".to_string());
            self.osc52_copy(&parent);
            self.status_message = Some(format!("[yank] {}", parent));
        }
    }

    /// Open the yank format picker overlay.
    ///
    /// Does nothing when the directory is empty (no entry to yank).
    pub fn open_yank_picker(&mut self) {
        if self.entries.get(self.selected).is_some() {
            self.yank_picker_mode = true;
        }
    }

    /// Close the yank format picker without copying anything.
    pub fn close_yank_picker(&mut self) {
        self.yank_picker_mode = false;
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
