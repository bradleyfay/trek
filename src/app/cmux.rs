use super::opener::{default_rules, OpenerConfig};
/// cmux integration — open files in a new surface or via a configured opener.
///
/// File-open routing is now driven by the opener config
/// (`~/.config/trek/opener.conf`).  When a user config is present, commands
/// are read from it and executed via `sh -c`.  When no config exists the
/// built-in defaults from `opener::default_rules` are used, which replicates
/// the previous hardcoded behaviour:
///
/// - HTML / images / PDFs → system default opener (`open` / `xdg-open`)
/// - All other text/code  → new cmux terminal surface running `$EDITOR`
///
/// When Trek is not running inside cmux and the built-in `$EDITOR` fallback
/// applies, a status-bar hint is shown instead.
use super::App;

impl App {
    /// Open the selected file using the configured opener rules.
    ///
    /// Resolution order:
    /// 1. User opener config (`~/.config/trek/opener.conf`) — first match wins.
    /// 2. Built-in defaults — system open for binary types, `$EDITOR` in a new
    ///    cmux surface for code/text.
    ///
    /// Does nothing when the selected entry is a directory.
    pub fn open_in_cmux_tab(&mut self) {
        let entry = match self.entries.get(self.selected).cloned() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };

        // Prefer user config; fall back to built-in rules.
        let config = OpenerConfig::load().unwrap_or_else(|| OpenerConfig {
            rules: default_rules(),
        });

        let command_template = match config.find_command(&entry.path) {
            Some(t) => t.to_owned(),
            None => {
                self.status_message = Some(format!("No opener rule matched: {}", entry.name));
                return;
            }
        };

        let expanded = OpenerConfig::expand_command(&command_template, &entry.path);

        // Built-in `$EDITOR {}` — use cmux surface creation so the editor gets
        // an interactive terminal.  All other commands (including user-config
        // rules) are spawned via `sh -c` directly.
        if command_template == "$EDITOR {}" {
            self.open_in_editor_surface(&entry.name, &entry.path.to_string_lossy());
        } else {
            self.spawn_opener_command(&entry.name, &expanded);
        }
    }

    /// Execute `command` via `sh -c` as a background subprocess.
    ///
    /// Used for all user-configured opener rules and for system-open commands
    /// from the built-in defaults.
    fn spawn_opener_command(&mut self, name: &str, command: &str) {
        match std::process::Command::new("sh")
            .args(["-c", command])
            .spawn()
        {
            Ok(_) => {
                self.status_message = Some(format!("Opening: {}", name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to open {}: {}", name, e));
            }
        }
    }

    /// Open `path` in a new cmux terminal surface running `$EDITOR`.
    ///
    /// Shows a status-bar hint when Trek is not running inside cmux.
    fn open_in_editor_surface(&mut self, name: &str, path: &str) {
        if std::env::var("CMUX_WORKSPACE_ID").is_err() {
            self.status_message = Some("Not in cmux — use 'o' to open in editor".to_string());
            return;
        }

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());
        let command = format!("{} {}", editor, shell_escape(path));

        match std::process::Command::new("cmux")
            .args(["new-surface", "--type", "terminal"])
            .output()
        {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // Output format: "OK surface:N pane:N workspace:N"
                let surface_ref = stdout
                    .split_whitespace()
                    .find(|s| s.starts_with("surface:"))
                    .unwrap_or("")
                    .to_string();

                if surface_ref.is_empty() {
                    self.status_message = Some("cmux: could not parse surface ref".to_string());
                    return;
                }

                match std::process::Command::new("cmux")
                    .args(["send", "--surface", &surface_ref, &format!("{}\r", command)])
                    .status()
                {
                    Ok(_) => {
                        self.status_message = Some(format!("Opened in new tab: {}", name));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("cmux send failed: {}", e));
                    }
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.status_message = Some(format!("cmux error: {}", stderr.trim()));
            }
            Err(e) => {
                self.status_message = Some(format!("cmux not available: {}", e));
            }
        }
    }
}

/// Escape a path for safe use in a shell command string sent via `cmux send`.
fn shell_escape(s: &str) -> String {
    if s.contains([' ', '\'', '"', '\\', '(', ')', '[', ']', '{', '}', '&', ';']) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given: a path with a space in it
    /// When: shell_escape is called
    /// Then: the path is wrapped in single quotes
    #[test]
    fn shell_escape_wraps_paths_with_spaces() {
        let result = shell_escape("/home/user/my file.txt");
        assert_eq!(result, "'/home/user/my file.txt'");
    }

    /// Given: a path with no special characters
    /// When: shell_escape is called
    /// Then: the path is returned unchanged
    #[test]
    fn shell_escape_leaves_clean_paths_unchanged() {
        let result = shell_escape("/home/user/file.txt");
        assert_eq!(result, "/home/user/file.txt");
    }

    /// Given: a path with a single quote
    /// When: shell_escape is called
    /// Then: the single quote is escaped correctly
    #[test]
    fn shell_escape_handles_single_quote() {
        let result = shell_escape("/home/user/it's a file.txt");
        assert_eq!(result, "'/home/user/it'\\''s a file.txt'");
    }
}
