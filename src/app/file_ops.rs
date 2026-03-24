use super::App;
use crate::ops::{self, Clipboard, ClipboardOp};
use std::path::PathBuf;

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
        self.clipboard = Some(Clipboard {
            op: ClipboardOp::Copy,
            paths,
        });
        self.rename_selected.clear();
        self.status_message = Some(format!("[copy] {} files", count));
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
}
