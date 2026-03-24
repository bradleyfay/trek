use super::{App, DirEntry};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

impl App {
    /// Enter archive browsing mode for `archive_path`.
    ///
    /// Populates `self.entries` with the archive root contents and sets
    /// `archive_mode = true`.  Navigation (l/h) is handled by
    /// [`archive_enter_dir`] and [`archive_go_up`] instead of the normal
    /// filesystem navigation.
    pub fn enter_archive(&mut self, archive_path: PathBuf) {
        self.archive_path = Some(archive_path);
        self.archive_virt_dir = String::new();
        self.archive_mode = true;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_archive_dir();
    }

    /// Exit archive browsing mode and restore the real directory listing.
    pub fn exit_archive(&mut self) {
        self.archive_mode = false;
        self.archive_path = None;
        self.archive_virt_dir = String::new();
        self.archive_flat_paths.clear();
        self.load_dir();
        // Try to restore selection to the archive file.
        self.load_preview();
    }

    /// Navigate into a virtual subdirectory inside the archive.
    pub fn archive_enter_dir(&mut self, dir_name: String) {
        let new_virt = format!("{}{}/", self.archive_virt_dir, dir_name);
        self.archive_virt_dir = new_virt;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_archive_dir();
    }

    /// Navigate up one level inside the archive.
    ///
    /// If already at the root, exits archive mode entirely.
    pub fn archive_go_up(&mut self) {
        if self.archive_virt_dir.is_empty() {
            // At root — exit archive mode.
            self.exit_archive();
            return;
        }
        // Pop the last path component (strip trailing slash then find previous slash).
        let without_trailing = self.archive_virt_dir.trim_end_matches('/');
        let parent = match without_trailing.rfind('/') {
            Some(pos) => without_trailing[..=pos].to_string(),
            None => String::new(),
        };
        self.archive_virt_dir = parent;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_archive_dir();
    }

    /// Load (or refresh) the archive flat path list and rebuild `self.entries`
    /// for the current `archive_virt_dir`.
    pub fn load_archive_dir(&mut self) {
        let archive_path = match &self.archive_path {
            Some(p) => p.clone(),
            None => return,
        };

        // Refresh the flat path list if empty (first call after enter_archive).
        if self.archive_flat_paths.is_empty() {
            self.archive_flat_paths = crate::archive::list_archive_paths(&archive_path);
        }

        let prefix = &self.archive_virt_dir;
        let mut seen_dirs: HashSet<String> = HashSet::new();
        let mut entries: Vec<DirEntry> = Vec::new();

        for raw in &self.archive_flat_paths {
            // Strip the current virtual dir prefix.
            let rest = match raw.strip_prefix(prefix.as_str()) {
                Some(r) => r,
                None => continue,
            };
            if rest.is_empty() {
                // The entry IS the current directory — skip it.
                continue;
            }

            // Does it contain a '/' after stripping the prefix?
            if let Some(slash) = rest.find('/') {
                // It's a file inside a subdirectory (or the subdirectory itself).
                let dir_name = &rest[..slash];
                if !dir_name.is_empty() && seen_dirs.insert(dir_name.to_string()) {
                    entries.push(make_virtual_dir(dir_name, &archive_path));
                }
            } else {
                // Direct file child at this level.
                entries.push(make_virtual_file(rest, &archive_path, prefix));
            }
        }

        // Sort: directories first, then files, each group alphabetically.
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        self.entries = entries;
        self.entries_truncated = false;

        // Clear git status — not meaningful inside an archive.
        self.git_status = None;

        // Refresh preview for the newly selected entry.
        self.load_preview();
    }

    /// Handle `l`/`Enter` when in archive mode.
    ///
    /// - Directory entries → navigate into them via [`archive_enter_dir`].
    /// - File entries → extract to a temporary directory and load a text preview.
    pub fn archive_enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.is_dir {
                self.archive_enter_dir(entry.name.clone());
            } else {
                // Extract and preview the file.
                self.archive_extract_and_preview(entry.name.clone());
            }
        }
    }

    /// Extract the named file from the current virtual directory to a temp
    /// location and trigger an async preview of it.
    fn archive_extract_and_preview(&mut self, file_name: String) {
        let archive_path = match &self.archive_path {
            Some(p) => p.clone(),
            None => return,
        };
        let virt_path = format!("{}{}", self.archive_virt_dir, file_name);

        // Determine archive type and extract with the right tool.
        let tmp_dir = std::env::temp_dir().join(format!("trek_arc_prev_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp_dir);

        // Try zip crate first (works for .zip / .jar / .war / .ear).
        if let Some(extracted) =
            crate::archive::extract_zip_entry(&archive_path, &virt_path, &tmp_dir)
        {
            // Navigate to the temp dir so load_preview picks up the file.
            // We temporarily hijack preview by pointing at the extracted file.
            // Use a one-shot fake: override preview_lines directly after load.
            let old_cwd = self.cwd.clone();
            self.cwd = tmp_dir.clone();
            self.load_dir();
            if let Some(idx) = self.entries.iter().position(|e| e.path == extracted) {
                self.selected = idx;
            }
            self.load_preview();
            // Restore cwd (archive nav state is preserved in archive_path / virt_dir).
            self.cwd = old_cwd;
        } else {
            // For tar archives, extract via subprocess.
            let archive_str = archive_path.to_string_lossy();
            let tmp_str = tmp_dir.to_string_lossy();
            let result = std::process::Command::new("tar")
                .args([
                    "--extract",
                    "--file",
                    archive_str.as_ref(),
                    "--directory",
                    tmp_str.as_ref(),
                    "--strip-components=0",
                    virt_path.as_str(),
                ])
                .status();
            if result.map(|s| s.success()).unwrap_or(false) {
                let extracted = tmp_dir.join(&file_name);
                if extracted.exists() {
                    let old_cwd = self.cwd.clone();
                    self.cwd = tmp_dir.clone();
                    self.load_dir();
                    if let Some(idx) = self.entries.iter().position(|e| e.name == file_name) {
                        self.selected = idx;
                    }
                    self.load_preview();
                    self.cwd = old_cwd;
                }
            } else {
                self.status_message =
                    Some(format!("Cannot preview {} — extraction failed", file_name));
            }
        }
    }

    /// Return the display breadcrumb string for the current archive location.
    ///
    /// e.g. `"archive.zip / src / utils"` when browsing `src/utils/` inside
    /// `archive.zip`.
    pub fn archive_breadcrumb(&self) -> String {
        let archive_name = self
            .archive_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "archive".to_string());

        if self.archive_virt_dir.is_empty() {
            archive_name
        } else {
            let parts: Vec<&str> = self
                .archive_virt_dir
                .trim_end_matches('/')
                .split('/')
                .collect();
            format!("{} / {}", archive_name, parts.join(" / "))
        }
    }
}

// ── Virtual DirEntry construction ─────────────────────────────────────────

fn make_virtual_dir(name: &str, archive_path: &Path) -> DirEntry {
    DirEntry {
        name: name.to_string(),
        // Virtual path: we embed the archive path so the UI can display it.
        // The path does not exist on disk; all navigation uses archive_nav methods.
        path: archive_path.join(name),
        is_dir: true,
        size: 0,
        modified: SystemTime::UNIX_EPOCH,
        child_count: None,
    }
}

fn make_virtual_file(name: &str, archive_path: &Path, virt_dir: &str) -> DirEntry {
    DirEntry {
        name: name.to_string(),
        path: archive_path.join(format!("{}{}", virt_dir, name)),
        is_dir: false,
        size: 0,
        modified: SystemTime::UNIX_EPOCH,
        child_count: None,
    }
}
