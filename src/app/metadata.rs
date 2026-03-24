use super::{format_permission_bits, format_unix_timestamp_utc, meta_human_size, App};
use std::path::Path;

impl App {
    /// Toggle between content preview and metadata preview.
    pub fn toggle_meta_preview(&mut self) {
        self.meta_preview_mode = !self.meta_preview_mode;
        if self.meta_preview_mode {
            self.diff_preview_mode = false; // mutually exclusive
        }
        self.load_preview();
    }

    /// Build the metadata card lines for `path`.
    pub fn load_meta_lines(path: &Path) -> Vec<String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let meta = match std::fs::symlink_metadata(path) {
                Ok(m) => m,
                Err(e) => return vec![String::new(), format!("  Error reading metadata: {}", e)],
            };
            let size = meta.len();
            let file_type = if meta.is_dir() {
                "Directory"
            } else if meta.file_type().is_symlink() {
                "Symbolic link"
            } else {
                "Regular file"
            };
            let mode = meta.permissions().mode();
            let fmt_time = |st: std::io::Result<std::time::SystemTime>| -> String {
                st.ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| format_unix_timestamp_utc(d.as_secs()))
                    .unwrap_or_else(|| "unavailable".to_string())
            };
            vec![
                String::new(),
                format!("  Path      {}", path.display()),
                format!("  Type      {}", file_type),
                format!("  Size      {} ({} bytes)", meta_human_size(size), size),
                format!(
                    "  Mode      {} ({:04o})",
                    format_permission_bits(mode),
                    mode & 0o7777
                ),
                format!("  UID / GID {} / {}", meta.uid(), meta.gid()),
                format!("  Modified  {}", fmt_time(meta.modified())),
                format!("  Accessed  {}", fmt_time(meta.accessed())),
            ]
        }
        #[cfg(not(unix))]
        {
            vec![
                String::new(),
                format!("  Path     {}", path.display()),
                "  (Full metadata not available on this platform)".to_string(),
            ]
        }
    }

    // --- chmod editor (P) ---

    /// Open the chmod input bar for the currently selected entry.
    pub fn begin_chmod(&mut self) {
        if self.entries.get(self.selected).is_none() {
            return;
        }
        self.chmod_mode = true;
        self.chmod_input.clear();
    }

    /// Cancel the chmod input bar without making changes.
    pub fn cancel_chmod(&mut self) {
        self.chmod_mode = false;
        self.chmod_input.clear();
        self.status_message = Some("chmod cancelled".to_string());
    }

    /// Append an octal digit (max 4 chars).
    pub fn chmod_push_char(&mut self, c: char) {
        if self.chmod_input.len() < 4 {
            self.chmod_input.push(c);
        }
    }

    /// Remove the last digit from the chmod input.
    pub fn chmod_pop_char(&mut self) {
        self.chmod_input.pop();
    }

    /// Validate and apply the typed octal mode to the selected entry.
    pub fn confirm_chmod(&mut self) {
        let input = std::mem::take(&mut self.chmod_input);
        self.chmod_mode = false;

        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };
        let path = entry.path.clone();

        if input.is_empty() {
            self.status_message = Some("chmod cancelled (empty input)".to_string());
            return;
        }

        let mode = match u32::from_str_radix(input.trim(), 8) {
            Ok(m) if m <= 0o7777 => m,
            _ => {
                self.status_message = Some(format!("Invalid octal mode: '{}'", input));
                return;
            }
        };

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            match std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)) {
                Ok(()) => {
                    self.status_message = Some(format!("Mode set to {:04o}", mode));
                    if self.meta_preview_mode {
                        self.load_preview();
                    }
                }
                Err(e) => self.status_message = Some(format!("chmod failed: {}", e)),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
            self.status_message = Some("chmod is not supported on this platform".to_string());
        }
    }
}
