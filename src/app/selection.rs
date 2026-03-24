use super::App;
use std::fs;
use std::path::PathBuf;

impl App {
    /// Toggle the selection mark on entry `idx`.
    ///
    /// Directories are silently skipped.
    pub fn toggle_selection(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx) {
            if entry.is_dir {
                self.status_message = Some("Directory selection not supported".to_string());
                return;
            }
        }
        if self.rename_selected.contains(&idx) {
            self.rename_selected.remove(&idx);
        } else {
            self.rename_selected.insert(idx);
        }
    }

    /// Mark all non-directory entries in the current directory.
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

    /// Move cursor down while extending the selection (J key).
    ///
    /// Marks the current entry, moves down, and marks the new current entry.
    /// At the bottom of the list the cursor stays and the last entry is marked.
    pub fn select_move_down(&mut self) {
        self.rename_selected.insert(self.selected);
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
        self.rename_selected.insert(self.selected);
        self.load_preview();
    }

    /// Move cursor up while extending the selection (K key).
    ///
    /// Mirrors `select_move_down`. At the top of the list the cursor stays
    /// and the first entry is marked.
    pub fn select_move_up(&mut self) {
        self.rename_selected.insert(self.selected);
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.rename_selected.insert(self.selected);
        self.load_preview();
    }

    /// Read the text preview lines for a file at `path`.
    ///
    /// Returns a flat list of strings, one per display line.
    /// Handles archives, binary files, and oversized files gracefully.
    pub fn read_file_preview(path: &PathBuf) -> Vec<String> {
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
}
