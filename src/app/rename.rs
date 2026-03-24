use super::{App, DirEntry};
use crate::rename::{self, RenameField};
use std::fs;
use std::path::PathBuf;

/// Convert a simple glob pattern (`*`, `?` metacharacters) to a full-match regex string.
///
/// - `*`  matches any sequence of characters (including empty).
/// - `?`  matches exactly one character.
/// - All other characters are regex-escaped so that `.` in `*.tar.gz` matches a
///   literal dot and not "any character".
fn glob_to_regex(pattern: &str) -> String {
    let mut re = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            c => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    re
}

impl App {
    /// Toggle the rename-selection mark on entry `idx`.
    ///
    /// Directories are silently skipped (directory rename is out of scope for v1).
    pub fn toggle_selection(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx) {
            if entry.is_dir {
                self.status_message = Some("Directory rename not supported".to_string());
                return;
            }
        }
        if self.rename_selected.contains(&idx) {
            self.rename_selected.remove(&idx);
        } else {
            self.rename_selected.insert(idx);
        }
    }

    /// Mark all non-directory entries in the current directory for renaming.
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
    /// All entry types (including directories) are added to the selection —
    /// callers that only operate on files (e.g. start_rename) filter at their
    /// own boundary. At the bottom of the list the cursor stays and the last
    /// entry is marked.
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

    /// Enter rename mode (requires at least one *file* to be selected).
    ///
    /// Range selection (J/K) can add directories to `rename_selected`.
    /// Directories are skipped by the rename logic, so only count files.
    pub fn start_rename(&mut self) {
        let file_count = self
            .rename_selected
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .filter(|e| !e.is_dir)
            .count();
        if file_count == 0 {
            self.status_message = Some(if self.rename_selected.is_empty() {
                "No files selected".to_string()
            } else {
                "No files selected (directories cannot be renamed in bulk)".to_string()
            });
            return;
        }
        self.rename_mode = true;
        self.rename_pattern.clear();
        self.rename_replacement.clear();
        self.rename_focus = RenameField::Pattern;
        self.rename_preview.clear();
        self.rename_error = None;
        self.current_scroll = 0;
    }

    /// Exit rename mode without touching the filesystem.
    pub fn cancel_rename(&mut self) {
        self.rename_mode = false;
        self.rename_pattern.clear();
        self.rename_replacement.clear();
        self.rename_preview.clear();
        self.rename_error = None;
        self.rename_selected.clear();
        self.status_message = None;
    }

    /// Apply the current rename preview to the filesystem.
    pub fn confirm_rename(&mut self) {
        match rename::apply_renames(&self.rename_preview, &self.cwd) {
            Ok(count) => {
                let msg = format!(
                    "Renamed {} file{}",
                    count,
                    if count == 1 { "" } else { "s" }
                );
                self.rename_mode = false;
                self.rename_selected.clear();
                self.rename_pattern.clear();
                self.rename_replacement.clear();
                self.rename_preview.clear();
                self.rename_error = None;
                self.load_dir();
                self.status_message = Some(msg);
            }
            Err(e) => {
                self.rename_error = Some(e);
            }
        }
    }

    pub fn rename_push_char(&mut self, c: char) {
        match self.rename_focus {
            RenameField::Pattern => self.rename_pattern.push(c),
            RenameField::Replacement => self.rename_replacement.push(c),
        }
        self.update_rename_preview();
    }

    pub fn rename_pop_char(&mut self) {
        match self.rename_focus {
            RenameField::Pattern => {
                self.rename_pattern.pop();
            }
            RenameField::Replacement => {
                self.rename_replacement.pop();
            }
        }
        self.update_rename_preview();
    }

    pub fn rename_next_field(&mut self) {
        self.rename_focus = RenameField::Replacement;
    }

    pub fn rename_prev_field(&mut self) {
        self.rename_focus = RenameField::Pattern;
    }

    /// Recompute the live rename preview from the current pattern and replacement.
    fn update_rename_preview(&mut self) {
        let mut indices: Vec<usize> = self.rename_selected.iter().copied().collect();
        indices.sort_unstable();
        let selected_entries: Vec<&DirEntry> = indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .collect();
        let (preview, error) = rename::compute_preview(
            &selected_entries,
            &self.entries,
            &self.rename_pattern,
            &self.rename_replacement,
        );
        self.rename_preview = preview;
        self.rename_error = error;
    }

    // --- Glob pattern selection (*) ---

    /// Open the glob pattern input bar.
    pub fn begin_glob_select(&mut self) {
        self.glob_mode = true;
        self.glob_input.clear();
    }

    /// Close the glob bar without modifying the selection.
    pub fn cancel_glob_select(&mut self) {
        self.glob_mode = false;
        self.glob_input.clear();
    }

    /// Apply the glob pattern and add matching entries to `rename_selected`.
    ///
    /// - Empty pattern → silently close the bar (no-op).
    /// - Non-matching pattern → show a status message; keep bar closed.
    /// - Matching pattern → union-add matched indices and close the bar.
    pub fn confirm_glob_select(&mut self) {
        if self.glob_input.is_empty() {
            self.glob_mode = false;
            return;
        }

        let pattern = self.glob_input.clone();
        let re_str = glob_to_regex(&pattern);
        let re = match regex::Regex::new(&re_str) {
            Ok(r) => r,
            Err(_) => {
                self.status_message = Some(format!("Invalid glob pattern: {pattern}"));
                self.glob_mode = false;
                self.glob_input.clear();
                return;
            }
        };

        let matched: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir && re.is_match(&e.name))
            .map(|(i, _)| i)
            .collect();

        if matched.is_empty() {
            self.status_message = Some(format!("No entries match: {pattern}"));
        } else {
            let count = matched.len();
            for idx in matched {
                self.rename_selected.insert(idx);
            }
            self.status_message = Some(format!("Selected {count} file(s) matching \"{pattern}\""));
        }

        self.glob_mode = false;
        self.glob_input.clear();
    }

    /// Append a character to the glob pattern input.
    pub fn glob_push_char(&mut self, c: char) {
        self.glob_input.push(c);
    }

    /// Remove the last character from the glob pattern input.
    pub fn glob_pop_char(&mut self) {
        self.glob_input.pop();
    }

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
        // Previously we allocated the full buffer and then discarded it — this
        // avoids that wasted allocation and speeds up rejection of large files.
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
