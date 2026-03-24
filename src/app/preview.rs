use super::App;
use crate::icons::icon_for_entry;
use std::path::Path;
use std::process::Command;

impl App {
    pub fn load_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview_lines.clear();
        self.preview_is_diff = false;

        // Metadata card takes priority over content/diff views.
        if self.meta_preview_mode {
            if let Some(entry) = self.entries.get(self.selected).cloned() {
                self.preview_lines = Self::load_meta_lines(&entry.path);
            }
            return;
        }

        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
                // Directories never show a diff preview.
                if let Ok((children, _)) = Self::read_entries(
                    &entry.path,
                    self.show_hidden,
                    self.sort_mode,
                    self.sort_order,
                ) {
                    self.preview_lines = children
                        .iter()
                        .map(|c| {
                            let icon = icon_for_entry(&c.name, c.is_dir);
                            format!("{} {}", icon, c.name)
                        })
                        .collect();
                }
            } else if self.diff_preview_mode {
                // Show git diff if the file has changes; fall back to raw preview.
                let has_git_change = self
                    .git_status
                    .as_ref()
                    .and_then(|g| g.for_path(&entry.path))
                    .is_some();
                if has_git_change {
                    let diff = Self::load_git_diff(&entry.path);
                    if !diff.is_empty() {
                        self.preview_lines = diff;
                        self.preview_is_diff = true;
                        return;
                    }
                }
                // No diff available — fall back to raw content.
                self.preview_lines = Self::read_file_preview(&entry.path);
            } else {
                self.preview_lines = Self::read_file_preview(&entry.path);
            }
        }
    }

    /// Load `git diff` (unstaged, then staged) for `path` as a list of lines.
    fn load_git_diff(path: &Path) -> Vec<String> {
        let parent = match path.parent() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let path_str = path.to_string_lossy();

        // Try unstaged diff first.
        if let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["diff", "--no-color", "--", path_str.as_ref()])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .take(2000)
                    .map(|l| l.to_string())
                    .collect();
            }
        }

        // Fall back to staged diff.
        if let Ok(out) = Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["diff", "--cached", "--no-color", "--", path_str.as_ref()])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .take(2000)
                    .map(|l| l.to_string())
                    .collect();
            }
        }

        Vec::new()
    }

    /// Toggle diff preview mode for the currently selected file.
    ///
    /// Has no effect outside a git repo or when the selected item is a
    /// directory or a clean (unmodified) file.
    pub fn toggle_diff_preview(&mut self) {
        let has_git_change = self
            .entries
            .get(self.selected)
            .filter(|e| !e.is_dir)
            .and_then(|e| self.git_status.as_ref().and_then(|g| g.for_path(&e.path)))
            .is_some();

        if has_git_change {
            self.diff_preview_mode = !self.diff_preview_mode;
            if self.diff_preview_mode {
                self.meta_preview_mode = false; // mutually exclusive
            }
            self.load_preview();
        } else {
            self.status_message = Some("No git changes for this file".to_string());
        }
    }

    /// Re-run `git status` for the current directory and refresh the preview.
    pub fn refresh_git_status(&mut self) {
        self.git_status = crate::git::GitStatus::load(&self.cwd);
        self.load_preview();
        self.status_message = Some("Git status refreshed".to_string());
    }
}
