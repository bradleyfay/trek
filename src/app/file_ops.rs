use super::App;
use crate::ops::{self, Clipboard, ClipboardOp};
use std::path::PathBuf;

/// Suggest a duplicate name for `name` by inserting `_copy` before the first extension.
///
/// Uses `split_once` to avoid raw byte-offset indexing, which is safe for any
/// valid UTF-8 filename (including those with multi-byte characters before the dot).
///
/// Examples:
///   "config.toml"    → "config_copy.toml"
///   "archive.tar.gz" → "archive_copy.tar.gz"  (preserves compound extension)
///   "Makefile"       → "Makefile_copy"
///   "café.txt"       → "café_copy.txt"
fn suggest_dup_name(name: &str) -> String {
    if let Some((stem, ext)) = name.split_once('.') {
        if !stem.is_empty() {
            return format!("{}_copy.{}", stem, ext);
        }
    }
    format!("{}_copy", name)
}

impl App {
    /// Yank (copy) the currently selected entry to the clipboard.
    pub fn clipboard_copy_current(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
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
        // Compute total size of selected files before clearing selection.
        let total_bytes: u64 = self
            .nav
            .selection
            .iter()
            .filter_map(|&i| self.nav.entries.get(i))
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        self.clipboard = Some(Clipboard {
            op: ClipboardOp::Copy,
            paths,
        });
        self.nav.selection.clear();
        let size_str = if total_bytes > 0 {
            format!(" ({})", crate::app::format_size(total_bytes))
        } else {
            String::new()
        };
        self.status_message = Some(format!("[copy] {} files{}", count, size_str));
    }

    /// Mark the currently selected entry for cut.
    pub fn clipboard_cut_current(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
            self.clipboard = Some(Clipboard {
                op: ClipboardOp::Cut,
                paths: vec![entry.path.clone()],
            });
            self.status_message = Some(format!("[cut] \"{}\"", entry.name));
        }
    }

    /// Begin a delete confirmation for the currently selected entry.
    pub fn begin_delete_current(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
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

    /// Move pending files to the platform trash (recoverable).
    pub fn confirm_trash(&mut self) {
        let paths = std::mem::take(&mut self.pending_delete);
        let mut done = 0usize;
        let mut errors: Vec<String> = Vec::new();
        let mut trashed: Vec<crate::trash::TrashedEntry> = Vec::new();
        for path in &paths {
            match crate::trash::trash_path(path) {
                Ok(entry) => {
                    done += 1;
                    trashed.push(entry);
                }
                Err(e) => errors.push(e.to_string()),
            }
        }
        if !trashed.is_empty() {
            self.last_trashed = trashed;
        }
        self.nav.selection.clear();
        if let Some(err) = errors.first() {
            self.status_message = Some(format!("Error: {}", err));
        } else {
            self.status_message = Some(format!(
                "Trashed {} item{} [u to undo]",
                done,
                if done == 1 { "" } else { "s" }
            ));
        }
        self.load_dir();
        self.git_status = crate::git::GitStatus::load(&self.nav.cwd);
    }

    /// Permanently delete the pending files (no recovery).
    pub fn confirm_permanent_delete(&mut self) {
        let paths = std::mem::take(&mut self.pending_delete);
        let mut done = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for path in &paths {
            match ops::delete_path(path) {
                Ok(()) => done += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }
        self.nav.selection.clear();
        if let Some(err) = errors.first() {
            self.status_message = Some(format!("Error: {}", err));
        } else {
            self.status_message = Some(format!(
                "Permanently deleted {} item{}",
                done,
                if done == 1 { "" } else { "s" }
            ));
        }
        self.load_dir();
        self.git_status = crate::git::GitStatus::load(&self.nav.cwd);
    }

    /// Restore the most recently trashed group back to their original paths.
    pub fn undo_trash(&mut self) {
        if self.last_trashed.is_empty() {
            self.status_message = Some("Nothing to undo".to_string());
            return;
        }
        let entries = std::mem::take(&mut self.last_trashed);
        let first_name = entries
            .first()
            .and_then(|e| e.original.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut restored = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for entry in &entries {
            match crate::trash::restore_path(entry) {
                Ok(()) => restored += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }
        if let Some(err) = errors.first() {
            self.status_message = Some(format!("Restore failed: {}", err));
        } else {
            self.status_message = Some(format!(
                "Restored: {}{}",
                first_name,
                if restored > 1 {
                    format!(" (+{} more)", restored - 1)
                } else {
                    String::new()
                }
            ));
        }
        self.load_dir();
        self.git_status = crate::git::GitStatus::load(&self.nav.cwd);
    }

    /// Cancel the pending deletion.
    pub fn cancel_delete(&mut self) {
        self.pending_delete.clear();
        self.status_message = Some("Delete cancelled".to_string());
    }

    /// Enter touch mode.
    pub fn begin_touch(&mut self) {
        self.overlay.touch_mode = true;
        self.overlay.touch_input.clear();
    }

    /// Cancel touch mode without creating anything.
    pub fn cancel_touch(&mut self) {
        self.overlay.touch_mode = false;
        self.overlay.touch_input.clear();
    }

    /// Execute touch with the current input and exit touch mode.
    pub fn confirm_touch(&mut self) {
        let name = self.overlay.touch_input.trim().to_string();
        self.overlay.touch_mode = false;
        self.overlay.touch_input.clear();
        if name.is_empty() {
            self.status_message = Some("File name cannot be empty".to_string());
            return;
        }
        match ops::touch_file(&self.nav.cwd, &name) {
            Ok(_) => {
                self.status_message = Some(format!("Created \"{}\"", name));
                self.load_dir();
                if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                    self.nav.selected = idx;
                    self.load_preview();
                }
            }
            Err(e) => {
                let msg = if e.to_string().contains("exists") {
                    format!("'{}' already exists", name)
                } else {
                    format!("touch failed: {}", e)
                };
                self.status_message = Some(msg);
            }
        }
    }

    pub fn touch_push_char(&mut self, c: char) {
        self.overlay.touch_input.push(c);
    }

    pub fn touch_pop_char(&mut self) {
        self.overlay.touch_input.pop();
    }

    /// Enter mkdir mode.
    pub fn begin_mkdir(&mut self) {
        self.overlay.mkdir_mode = true;
        self.overlay.mkdir_input.clear();
    }

    /// Execute mkdir with the current input and exit mkdir mode.
    pub fn confirm_mkdir(&mut self) {
        let name = self.overlay.mkdir_input.trim().to_string();
        self.overlay.mkdir_mode = false;
        self.overlay.mkdir_input.clear();
        if name.is_empty() {
            self.status_message = Some("Directory name cannot be empty".to_string());
            return;
        }
        match ops::make_dir(&self.nav.cwd, &name) {
            Ok(_) => {
                self.status_message = Some(format!("Created directory \"{}\"", name));
                self.load_dir();
                // Select the newly created directory.
                if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                    self.nav.selected = idx;
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
        self.overlay.mkdir_mode = false;
        self.overlay.mkdir_input.clear();
    }

    pub fn mkdir_push_char(&mut self, c: char) {
        self.overlay.mkdir_input.push(c);
    }

    pub fn mkdir_pop_char(&mut self) {
        self.overlay.mkdir_input.pop();
    }

    /// Return paths of all rename-selected entries, sorted by index.
    fn selected_paths(&self) -> Vec<PathBuf> {
        let mut indices: Vec<usize> = self.nav.selection.iter().copied().collect();
        indices.sort_unstable();
        indices
            .iter()
            .filter_map(|&i| self.nav.entries.get(i))
            .map(|e| e.path.clone())
            .collect()
    }

    // --- File duplication (W) ---

    /// Open the duplicate name bar for the currently selected entry.
    ///
    /// Pre-fills the input with a suggested name derived from the source name.
    /// Does nothing if the directory is empty.
    pub fn begin_dup(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
            self.overlay.dup_src = Some(entry.path.clone());
            self.overlay.dup_input = suggest_dup_name(&entry.name);
            self.overlay.dup_mode = true;
        }
    }

    /// Cancel the duplication without touching the filesystem.
    pub fn cancel_dup(&mut self) {
        self.overlay.dup_mode = false;
        self.overlay.dup_input.clear();
        self.overlay.dup_src = None;
    }

    /// Execute the duplication with the current input name.
    ///
    /// - Empty name → error message, bar closed.
    /// - Destination already exists → error message, no overwrite.
    /// - Success → copy created, listing refreshed, new entry selected.
    pub fn confirm_dup(&mut self) {
        let name = self.overlay.dup_input.trim().to_string();
        self.overlay.dup_mode = false;
        self.overlay.dup_input.clear();
        let src = match self.overlay.dup_src.take() {
            Some(p) => p,
            None => return,
        };
        if name.is_empty() {
            self.status_message = Some("Name cannot be empty".to_string());
            return;
        }
        let dst = self.nav.cwd.join(&name);
        if dst.exists() {
            self.status_message = Some(format!("'{}' already exists", name));
            return;
        }
        match ops::copy_path(&src, &dst) {
            Ok(()) => {
                self.load_dir();
                if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                    self.nav.selected = idx;
                    self.load_preview();
                }
                self.status_message = Some(format!("Duplicated \u{2192} \"{}\"", name));
            }
            Err(e) => {
                self.status_message = Some(format!("Duplicate failed: {}", e));
            }
        }
    }

    /// Append a character to the duplicate name input.
    pub fn dup_push_char(&mut self, c: char) {
        self.overlay.dup_input.push(c);
    }

    /// Remove the last character from the duplicate name input.
    pub fn dup_pop_char(&mut self) {
        self.overlay.dup_input.pop();
    }

    // --- Symlink creation (L) ---

    /// Enter symlink mode for the currently selected entry.
    ///
    /// Pre-fills the input with the selected entry's filename; stores
    /// the entry's absolute path as the link target.
    /// Does nothing when the directory is empty.
    pub fn begin_symlink(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
            self.overlay.symlink_target = Some(entry.path.clone());
            self.overlay.symlink_input = entry.name.clone();
            self.overlay.symlink_mode = true;
        }
    }

    /// Cancel symlink mode without touching the filesystem.
    pub fn cancel_symlink(&mut self) {
        self.overlay.symlink_mode = false;
        self.overlay.symlink_input.clear();
        self.overlay.symlink_target = None;
    }

    /// Execute symlink creation with the current input name.
    ///
    /// - Empty name → error message.
    /// - Name already exists (file, directory, or dangling symlink) → error message.
    /// - Success → symlink created, listing refreshed, new entry selected.
    /// - Non-Unix platforms → informational error message.
    pub fn confirm_symlink(&mut self) {
        let name = self.overlay.symlink_input.trim().to_string();
        self.overlay.symlink_mode = false;
        self.overlay.symlink_input.clear();
        let target = match self.overlay.symlink_target.take() {
            Some(p) => p,
            None => return,
        };
        if name.is_empty() {
            self.status_message = Some("Symlink name cannot be empty".to_string());
            return;
        }
        let link_path = self.nav.cwd.join(&name);
        // Use symlink_metadata to catch dangling symlinks that .exists() misses.
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            self.status_message = Some(format!("'{}' already exists", name));
            return;
        }
        #[cfg(unix)]
        match std::os::unix::fs::symlink(&target, &link_path) {
            Ok(()) => {
                self.load_dir();
                self.git_status = crate::git::GitStatus::load(&self.nav.cwd);
                if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                    self.nav.selected = idx;
                    self.load_preview();
                }
                self.status_message = Some(format!(
                    "Created symlink \"{}\" \u{2192} {}",
                    name,
                    target.to_string_lossy()
                ));
            }
            Err(e) => {
                self.status_message = Some(format!("symlink failed: {}", e));
            }
        }
        #[cfg(not(unix))]
        {
            self.status_message = Some("Symlink creation requires a Unix system".to_string());
        }
    }

    /// Append a character to the symlink name input.
    pub fn symlink_push_char(&mut self, c: char) {
        self.overlay.symlink_input.push(c);
    }

    /// Remove the last character from the symlink name input.
    pub fn symlink_pop_char(&mut self) {
        self.overlay.symlink_input.pop();
    }

    /// Open the clipboard inspector overlay.
    pub fn open_clipboard_inspect(&mut self) {
        self.overlay.clipboard_inspect_mode = true;
    }

    /// Close the clipboard inspector overlay without taking any action.
    pub fn close_clipboard_inspect(&mut self) {
        self.overlay.clipboard_inspect_mode = false;
    }

    // --- Archive extraction (Z) ---

    /// Begin an extraction confirmation for the currently selected entry.
    ///
    /// Shows `"Not an archive"` if the entry is not a recognized archive type.
    pub fn begin_extract(&mut self) {
        if let Some(entry) = self.nav.entries.get(self.nav.selected) {
            if crate::archive::is_archive(&entry.path) {
                self.pending_extract = Some(entry.path.clone());
            } else {
                self.status_message = Some("Not an archive".to_string());
            }
        }
    }

    /// Cancel the extraction without touching the filesystem.
    pub fn cancel_extract(&mut self) {
        self.pending_extract = None;
        self.status_message = Some("Extract cancelled".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::suggest_dup_name;

    #[test]
    fn dup_name_simple_extension() {
        assert_eq!(suggest_dup_name("config.toml"), "config_copy.toml");
    }

    #[test]
    fn dup_name_compound_extension() {
        assert_eq!(suggest_dup_name("archive.tar.gz"), "archive_copy.tar.gz");
    }

    #[test]
    fn dup_name_no_extension() {
        assert_eq!(suggest_dup_name("Makefile"), "Makefile_copy");
    }

    #[test]
    fn dup_name_dotfile_no_extension() {
        // Leading-dot names (e.g. ".hidden") are treated as no-stem — fall through.
        assert_eq!(suggest_dup_name(".hidden"), ".hidden_copy");
    }

    #[test]
    fn dup_name_multibyte_before_dot() {
        // Multi-byte UTF-8 characters in the stem must not cause a panic.
        assert_eq!(suggest_dup_name("café.txt"), "café_copy.txt");
        assert_eq!(suggest_dup_name("日本語.txt"), "日本語_copy.txt");
    }

    #[test]
    fn dup_name_multibyte_compound_extension() {
        assert_eq!(suggest_dup_name("données.tar.gz"), "données_copy.tar.gz");
    }

    #[test]
    fn dup_name_double_dot() {
        // ".." has an empty stem — treated as no-stem, gets _copy suffix.
        assert_eq!(suggest_dup_name(".."), ".._copy");
    }

    #[test]
    fn dup_name_purely_multibyte_no_dot() {
        // A name with only multi-byte chars and no dot is treated as no-extension.
        assert_eq!(suggest_dup_name("日本語"), "日本語_copy");
    }
}
