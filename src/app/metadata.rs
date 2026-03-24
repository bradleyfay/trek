use super::{format_permission_bits, format_unix_timestamp_utc, meta_human_size, App};
use std::path::Path;

/// Maximum file size for hash computation (512 MB).
/// SHA-256 of 512 MB completes in < 1 s on modern hardware; above this we skip.
const MAX_HASH_FILE_SIZE: u64 = 512 * 1024 * 1024;

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
    // --- Hash preview (H) ---

    /// Toggle between content preview and hash (SHA-256) preview.
    ///
    /// No-op if the selected entry is a directory — shows a status message instead.
    /// Clears `meta_preview_mode` and `diff_preview_mode` when entering hash mode.
    pub fn toggle_hash_preview(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.status_message =
                    Some("Hash preview not available for directories".to_string());
                return;
            }
        }
        self.hash_preview_mode = !self.hash_preview_mode;
        if self.hash_preview_mode {
            self.meta_preview_mode = false;
            self.diff_preview_mode = false;
            self.git_log_mode = false; // mutually exclusive
        }
        self.load_preview();
    }

    /// Compute the SHA-256 hash of `path` and return formatted preview lines.
    ///
    /// Uses `shasum -a 256` (macOS) or `sha256sum` (Linux/GNU coreutils).
    /// Returns an informational message if the file is too large, the tool is
    /// missing, or the output cannot be parsed.
    pub fn load_hash_lines(path: &Path) -> Vec<String> {
        // Size guard — avoid blocking the UI on huge files.
        match std::fs::metadata(path) {
            Ok(meta) if meta.len() > MAX_HASH_FILE_SIZE => {
                return vec![
                    String::new(),
                    format!(
                        "  File too large to hash ({} — limit 512 MB)",
                        meta_human_size(meta.len())
                    ),
                ];
            }
            Err(e) => return vec![String::new(), format!("  Error reading file: {}", e)],
            _ => {}
        }

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Detect available hashing tool.
        let tool_available = |bin: &str| -> bool {
            std::process::Command::new("which")
                .arg(bin)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        let (cmd, extra_args): (&str, &[&str]) = if tool_available("shasum") {
            ("shasum", &["-a", "256"])
        } else if tool_available("sha256sum") {
            ("sha256sum", &[])
        } else {
            return vec![
                String::new(),
                "  SHA-256 hash requires shasum or sha256sum".to_string(),
                String::new(),
                "  Install: brew install coreutils   (macOS)".to_string(),
                "           apt install coreutils    (Debian/Ubuntu)".to_string(),
            ];
        };

        let output = match std::process::Command::new(cmd)
            .args(extra_args)
            .arg(path)
            .output()
        {
            Ok(o) => o,
            Err(e) => return vec![String::new(), format!("  Failed to run {}: {}", cmd, e)],
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Both shasum and sha256sum output: "<64-char-hash>  <filename>"
        let hash = match stdout.split_whitespace().next() {
            Some(h) if h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) => h.to_string(),
            _ => {
                return vec![
                    String::new(),
                    "  Could not parse hash output".to_string(),
                    format!("  Raw: {}", stdout.trim()),
                ]
            }
        };

        let size_str = std::fs::metadata(path)
            .map(|m| meta_human_size(m.len()))
            .unwrap_or_else(|_| "unknown".to_string());

        vec![
            String::new(),
            format!("  SHA-256  {}", hash),
            String::new(),
            format!("  File     {}", file_name),
            format!("  Size     {}", size_str),
        ]
    }

    // --- Metadata preview (m) ---

    /// Toggle between content preview and metadata preview.
    pub fn toggle_meta_preview(&mut self) {
        self.meta_preview_mode = !self.meta_preview_mode;
        if self.meta_preview_mode {
            self.diff_preview_mode = false;
            self.hash_preview_mode = false; // mutually exclusive
            self.git_log_mode = false; // mutually exclusive
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
