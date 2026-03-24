use super::App;
use crate::ops::{self, Clipboard, ClipboardOp};
use std::path::PathBuf;

/// Suggest a duplicate name for `name` by inserting `_copy` before the last extension.
///
/// Examples:
///   "config.toml"    → "config_copy.toml"
///   "archive.tar.gz" → "archive_copy.tar.gz"  (preserves compound extension)
///   "Makefile"       → "Makefile_copy"
fn suggest_dup_name(name: &str) -> String {
    if let Some(dot) = name.find('.') {
        if dot > 0 {
            let stem = &name[..dot];
            let ext = &name[dot..]; // includes the leading dot and any compound extension
            return format!("{}_copy{}", stem, ext);
        }
    }
    format!("{}_copy", name)
}

impl App {
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
        // Compute total size of selected files before clearing rename_selected.
        let total_bytes: u64 = self
            .rename_selected
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        self.clipboard = Some(Clipboard {
            op: ClipboardOp::Copy,
            paths,
        });
        self.rename_selected.clear();
        let size_str = if total_bytes > 0 {
            format!(" ({})", crate::app::format_size(total_bytes))
        } else {
            String::new()
        };
        self.status_message = Some(format!("[copy] {} files{}", count, size_str));
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
        self.rename_selected.clear();
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
        self.git_status = crate::git::GitStatus::load(&self.cwd);
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
        self.rename_selected.clear();
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
        self.git_status = crate::git::GitStatus::load(&self.cwd);
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
        self.git_status = crate::git::GitStatus::load(&self.cwd);
    }

    /// Cancel the pending deletion.
    pub fn cancel_delete(&mut self) {
        self.pending_delete.clear();
        self.status_message = Some("Delete cancelled".to_string());
    }

    /// Enter touch mode.
    pub fn begin_touch(&mut self) {
        self.touch_mode = true;
        self.touch_input.clear();
    }

    /// Cancel touch mode without creating anything.
    pub fn cancel_touch(&mut self) {
        self.touch_mode = false;
        self.touch_input.clear();
    }

    /// Execute touch with the current input and exit touch mode.
    pub fn confirm_touch(&mut self) {
        let name = self.touch_input.trim().to_string();
        self.touch_mode = false;
        self.touch_input.clear();
        if name.is_empty() {
            self.status_message = Some("File name cannot be empty".to_string());
            return;
        }
        match ops::touch_file(&self.cwd, &name) {
            Ok(_) => {
                self.status_message = Some(format!("Created \"{}\"", name));
                self.load_dir();
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
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
        self.touch_input.push(c);
    }

    pub fn touch_pop_char(&mut self) {
        self.touch_input.pop();
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

    // --- File duplication (W) ---

    /// Open the duplicate name bar for the currently selected entry.
    ///
    /// Pre-fills the input with a suggested name derived from the source name.
    /// Does nothing if the directory is empty.
    pub fn begin_dup(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.dup_src = Some(entry.path.clone());
            self.dup_input = suggest_dup_name(&entry.name);
            self.dup_mode = true;
        }
    }

    /// Cancel the duplication without touching the filesystem.
    pub fn cancel_dup(&mut self) {
        self.dup_mode = false;
        self.dup_input.clear();
        self.dup_src = None;
    }

    /// Execute the duplication with the current input name.
    ///
    /// - Empty name → error message, bar closed.
    /// - Destination already exists → error message, no overwrite.
    /// - Success → copy created, listing refreshed, new entry selected.
    pub fn confirm_dup(&mut self) {
        let name = self.dup_input.trim().to_string();
        self.dup_mode = false;
        self.dup_input.clear();
        let src = match self.dup_src.take() {
            Some(p) => p,
            None => return,
        };
        if name.is_empty() {
            self.status_message = Some("Name cannot be empty".to_string());
            return;
        }
        let dst = self.cwd.join(&name);
        if dst.exists() {
            self.status_message = Some(format!("'{}' already exists", name));
            return;
        }
        match ops::copy_path(&src, &dst) {
            Ok(()) => {
                self.load_dir();
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
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
        self.dup_input.push(c);
    }

    /// Remove the last character from the duplicate name input.
    pub fn dup_pop_char(&mut self) {
        self.dup_input.pop();
    }

    // --- Symlink creation (L) ---

    /// Enter symlink mode for the currently selected entry.
    ///
    /// Pre-fills the input with the selected entry's filename; stores
    /// the entry's absolute path as the link target.
    /// Does nothing when the directory is empty.
    pub fn begin_symlink(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.symlink_target = Some(entry.path.clone());
            self.symlink_input = entry.name.clone();
            self.symlink_mode = true;
        }
    }

    /// Cancel symlink mode without touching the filesystem.
    pub fn cancel_symlink(&mut self) {
        self.symlink_mode = false;
        self.symlink_input.clear();
        self.symlink_target = None;
    }

    /// Execute symlink creation with the current input name.
    ///
    /// - Empty name → error message.
    /// - Name already exists (file, directory, or dangling symlink) → error message.
    /// - Success → symlink created, listing refreshed, new entry selected.
    /// - Non-Unix platforms → informational error message.
    pub fn confirm_symlink(&mut self) {
        let name = self.symlink_input.trim().to_string();
        self.symlink_mode = false;
        self.symlink_input.clear();
        let target = match self.symlink_target.take() {
            Some(p) => p,
            None => return,
        };
        if name.is_empty() {
            self.status_message = Some("Symlink name cannot be empty".to_string());
            return;
        }
        let link_path = self.cwd.join(&name);
        // Use symlink_metadata to catch dangling symlinks that .exists() misses.
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            self.status_message = Some(format!("'{}' already exists", name));
            return;
        }
        #[cfg(unix)]
        match std::os::unix::fs::symlink(&target, &link_path) {
            Ok(()) => {
                self.load_dir();
                self.git_status = crate::git::GitStatus::load(&self.cwd);
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
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
        self.symlink_input.push(c);
    }

    /// Remove the last character from the symlink name input.
    pub fn symlink_pop_char(&mut self) {
        self.symlink_input.pop();
    }

    /// Open the clipboard inspector overlay.
    pub fn open_clipboard_inspect(&mut self) {
        self.clipboard_inspect_mode = true;
    }

    /// Close the clipboard inspector overlay without taking any action.
    pub fn close_clipboard_inspect(&mut self) {
        self.clipboard_inspect_mode = false;
    }

    // --- Archive extraction (Z) ---

    /// Begin an extraction confirmation for the currently selected entry.
    ///
    /// Shows `"Not an archive"` if the entry is not a recognized archive type.
    pub fn begin_extract(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if crate::archive::is_archive(&entry.path) {
                self.pending_extract = Some(entry.path.clone());
            } else {
                self.status_message = Some("Not an archive".to_string());
            }
        }
    }

    /// Run the extraction after the user confirms with y/Enter.
    pub fn confirm_extract(&mut self) {
        let path = match self.pending_extract.take() {
            Some(p) => p,
            None => return,
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        match crate::archive::extract_archive(&path, &self.cwd) {
            Ok(()) => {
                self.status_message = Some(format!("Extracted: {}", name));
                self.load_dir();
                self.git_status = crate::git::GitStatus::load(&self.cwd);
            }
            Err(e) => {
                self.status_message = Some(format!("Extract failed: {}", e));
            }
        }
    }

    /// Cancel the extraction without touching the filesystem.
    pub fn cancel_extract(&mut self) {
        self.pending_extract = None;
        self.status_message = Some("Extract cancelled".to_string());
    }

    // ── Archive creation (E) ──────────────────────────────────────────────────

    /// Open the archive-creation input bar with a suggested filename.
    ///
    /// - 1 entry selected → `"<name>.tar.gz"`
    /// - Multiple selected → `"archive.tar.gz"`
    /// - No selection → current entry name + `".tar.gz"`
    pub fn begin_archive_create(&mut self) {
        let suggestion = if !self.rename_selected.is_empty() {
            if self.rename_selected.len() == 1 {
                let idx = *self.rename_selected.iter().next().unwrap();
                let stem = self
                    .entries
                    .get(idx)
                    .map(|e| e.name.as_str())
                    .unwrap_or("archive");
                format!("{}.tar.gz", stem)
            } else {
                "archive.tar.gz".to_string()
            }
        } else if let Some(entry) = self.entries.get(self.selected) {
            format!("{}.tar.gz", entry.name)
        } else {
            return; // No entries at all.
        };
        self.archive_create_input = suggestion;
        self.archive_create_mode = true;
    }

    /// Cancel the archive-creation input bar without creating anything.
    pub fn cancel_archive_create(&mut self) {
        self.archive_create_mode = false;
        self.archive_create_input.clear();
        self.status_message = Some("Archive creation cancelled".to_string());
    }

    /// Validate the typed name and create the archive.
    pub fn confirm_archive_create(&mut self) {
        let name = self.archive_create_input.trim().to_string();
        self.archive_create_mode = false;
        self.archive_create_input.clear();

        if name.is_empty() {
            return;
        }

        let output_path = self.cwd.join(&name);
        if output_path.exists() {
            self.status_message = Some(format!("'{}' already exists", name));
            return;
        }

        // Collect inputs: selected entries, or current entry if no selection.
        let inputs: Vec<PathBuf> = if !self.rename_selected.is_empty() {
            self.rename_selected
                .iter()
                .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
                .collect()
        } else {
            self.entries
                .get(self.selected)
                .map(|e| vec![e.path.clone()])
                .unwrap_or_default()
        };

        if inputs.is_empty() {
            return;
        }

        let n = inputs.len();
        match crate::archive::create_archive(&output_path, &inputs) {
            Ok(()) => {
                self.load_dir();
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.selected = idx;
                    self.load_preview();
                }
                self.status_message = Some(format!(
                    "Created {} ({} entr{})",
                    name,
                    n,
                    if n == 1 { "y" } else { "ies" }
                ));
            }
            Err(msg) => {
                self.status_message = Some(format!("Archive failed: {}", msg));
            }
        }
    }

    /// Append a character to the archive name input.
    pub fn archive_create_push_char(&mut self, c: char) {
        self.archive_create_input.push(c);
    }

    /// Remove the last character from the archive name input.
    pub fn archive_create_pop_char(&mut self) {
        self.archive_create_input.pop();
    }
}
