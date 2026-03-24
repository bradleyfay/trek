/// cmux integration — open files in new tab/surface.
///
/// Trek routes file opens to cmux when running inside a cmux workspace.
/// The routing follows the table in CLAUDE.md:
///   - HTML / images / PDFs → system default opener (open / xdg-open)
///   - All other text/code  → new cmux terminal surface running $EDITOR
///
/// When Trek is not running inside cmux, a status-bar hint is shown instead
/// so the user knows to use `o` for in-place editor access.
use super::App;
use std::path::Path;

/// Determines how to open a file based on its extension.
enum OpenTarget {
    /// Open in a new cmux terminal surface with $EDITOR.
    Editor,
    /// Open with the platform system-default opener (background process).
    SystemDefault,
}

fn route_file(path: &Path) -> OpenTarget {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "html" | "htm" | "pdf" | "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico"
        | "bmp" | "tiff" | "tif" => OpenTarget::SystemDefault,
        _ => OpenTarget::Editor,
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

impl App {
    /// Open the selected file in a new cmux tab.
    ///
    /// Routes by file type following the CLAUDE.md routing table. Does nothing
    /// when the selected entry is a directory. Shows a status-bar hint when
    /// Trek is not running inside cmux (CMUX_WORKSPACE_ID is unset).
    pub fn open_in_cmux_tab(&mut self) {
        let entry = match self.entries.get(self.selected).cloned() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };

        if std::env::var("CMUX_WORKSPACE_ID").is_err() {
            self.status_message = Some("Not in cmux — use 'o' to open in editor".to_string());
            return;
        }

        let path = entry.path.to_string_lossy().into_owned();

        match route_file(&entry.path) {
            OpenTarget::SystemDefault => {
                #[cfg(target_os = "macos")]
                let opener = "open";
                #[cfg(not(target_os = "macos"))]
                let opener = "xdg-open";

                match std::process::Command::new(opener).arg(&path).spawn() {
                    Ok(_) => {
                        self.status_message = Some(format!("Opening: {}", entry.name));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to open {}: {}", entry.name, e));
                    }
                }
            }
            OpenTarget::Editor => {
                let editor = std::env::var("VISUAL")
                    .or_else(|_| std::env::var("EDITOR"))
                    .unwrap_or_else(|_| "vi".to_string());
                let command = format!("{} {}", editor, shell_escape(&path));

                // Create a new terminal surface in the current cmux workspace.
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
                            self.status_message =
                                Some("cmux: could not parse surface ref".to_string());
                            return;
                        }

                        match std::process::Command::new("cmux")
                            .args(["send", "--surface", &surface_ref, &format!("{}\r", command)])
                            .status()
                        {
                            Ok(_) => {
                                self.status_message =
                                    Some(format!("Opened in new tab: {}", entry.name));
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    /// Given: an HTML file path
    /// When: route_file is called
    /// Then: returns SystemDefault
    #[test]
    fn route_file_html_uses_system_default() {
        let path = PathBuf::from("index.html");
        assert!(matches!(route_file(&path), OpenTarget::SystemDefault));
    }

    /// Given: a PDF file path
    /// When: route_file is called
    /// Then: returns SystemDefault
    #[test]
    fn route_file_pdf_uses_system_default() {
        let path = PathBuf::from("doc.pdf");
        assert!(matches!(route_file(&path), OpenTarget::SystemDefault));
    }

    /// Given: a PNG image file path
    /// When: route_file is called
    /// Then: returns SystemDefault
    #[test]
    fn route_file_image_uses_system_default() {
        let path = PathBuf::from("photo.png");
        assert!(matches!(route_file(&path), OpenTarget::SystemDefault));
    }

    /// Given: a Rust source file path
    /// When: route_file is called
    /// Then: returns Editor
    #[test]
    fn route_file_code_uses_editor() {
        let path = PathBuf::from("main.rs");
        assert!(matches!(route_file(&path), OpenTarget::Editor));
    }

    /// Given: a Markdown file path
    /// When: route_file is called
    /// Then: returns Editor (markdown opens in editor by default)
    #[test]
    fn route_file_markdown_uses_editor() {
        let path = PathBuf::from("README.md");
        assert!(matches!(route_file(&path), OpenTarget::Editor));
    }
}
