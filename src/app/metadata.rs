use super::{format_permission_bits, format_unix_timestamp_utc, meta_human_size, App};
use std::path::Path;

/// Maximum file size for synchronous content statistics (line/word/char count).
/// Files larger than this are skipped to avoid blocking the render loop.
const STAT_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MB

/// Count lines, whitespace-delimited words, and Unicode characters in `path`.
///
/// Returns `None` if the file cannot be opened, is not valid UTF-8 (binary),
/// or exceeds `STAT_SIZE_LIMIT`.
fn count_text_stats(path: &Path, size: u64) -> Option<(u64, u64, u64)> {
    if size > STAT_SIZE_LIMIT {
        return None;
    }
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut line_count: u64 = 0;
    let mut word_count: u64 = 0;
    let mut char_count: u64 = 0;
    for line_result in reader.lines() {
        // `lines()` returns Err on invalid UTF-8 — treat as binary, abort.
        let line = line_result.ok()?;
        line_count += 1;
        word_count += line.split_whitespace().count() as u64;
        char_count += line.chars().count() as u64 + 1; // +1 for the stripped newline
    }
    Some((line_count, word_count, char_count))
}

impl App {
    // --- Metadata preview (m) ---

    /// Toggle between content preview and metadata preview.
    pub fn toggle_meta_preview(&mut self) {
        self.meta_preview_mode = !self.meta_preview_mode;
        if self.meta_preview_mode {
            self.diff_preview_mode = false;
            self.git_log_mode = false; // mutually exclusive
            self.file_compare_mode = false; // mutually exclusive
            self.hex_view_mode = false; // mutually exclusive
            self.du_preview_mode = false; // mutually exclusive
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
            let mut lines = vec![
                String::new(),
                format!("  Path      {}", path.display()),
                format!("  Type      {}", file_type),
            ];

            // For symlinks: show stored target path and whether it resolves.
            if meta.file_type().is_symlink() {
                match std::fs::read_link(path) {
                    Ok(target) => {
                        let home = std::env::var("HOME").unwrap_or_default();
                        let raw = target.to_string_lossy().into_owned();
                        let display = if !home.is_empty() {
                            raw.replace(&home, "~")
                        } else {
                            raw
                        };
                        lines.push(format!("  Target    {}", display));
                        let valid = path.exists();
                        lines.push(format!(
                            "  Valid     {}",
                            if valid {
                                "\u{2713}  exists"
                            } else {
                                "\u{2717}  dangling"
                            }
                        ));
                    }
                    Err(_) => {
                        lines.push("  Target    (unreadable)".to_string());
                    }
                }
            }

            lines.extend([
                format!("  Size      {} ({} bytes)", meta_human_size(size), size),
                format!(
                    "  Mode      {} ({:04o})",
                    format_permission_bits(mode),
                    mode & 0o7777
                ),
                format!("  UID / GID {} / {}", meta.uid(), meta.gid()),
                format!("  Modified  {}", fmt_time(meta.modified())),
                format!("  Accessed  {}", fmt_time(meta.accessed())),
            ]);

            // Append content stats for regular (non-dir, non-symlink) text files.
            if !meta.is_dir() && !meta.file_type().is_symlink() {
                if let Some((lc, wc, cc)) = count_text_stats(path, size) {
                    lines.push(format!("  Lines     {}", lc));
                    lines.push(format!("  Words     {}", wc));
                    lines.push(format!("  Chars     {}", cc));
                }
            }

            lines
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
