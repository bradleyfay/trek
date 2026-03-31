use super::App;
use std::path::Path;
use std::process::Command;

/// The format the user has chosen for the context bundle.
#[allow(dead_code, clippy::enum_variant_names)]
pub enum ContextBundleFormat {
    PathsOnly,
    PathsAndContents,
    PathsAndDiff,
}

/// Map a file extension to the Markdown fenced-code-block language tag.
fn lang_tag(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescript",
        "js" => "javascript",
        "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "sh" | "bash" | "zsh" => "bash",
        "md" => "markdown",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "css" => "css",
        _ => "",
    }
}

/// Return the git diff for `path` against HEAD (unified=3).
///
/// Returns `None` when there is no diff (untracked, no changes, or git error).
fn git_diff_for_path(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    let path_str = path.to_string_lossy();

    // Try unstaged diff first.
    if let Ok(out) = Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["diff", "--unified=3", "HEAD", "--", path_str.as_ref()])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            return Some(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }

    None
}

impl App {
    /// Open the context bundle format picker overlay.
    ///
    /// Does nothing when the directory is empty (nothing to export).
    pub fn open_context_bundle_picker(&mut self) {
        if self.entries.get(self.selected).is_some() || !self.rename_selected.is_empty() {
            self.context_bundle_picker_mode = true;
        }
    }

    /// Close the context bundle format picker without exporting anything.
    pub fn close_context_bundle_picker(&mut self) {
        self.context_bundle_picker_mode = false;
    }

    /// Build and copy the context bundle in the requested format.
    ///
    /// File selection priority:
    /// 1. `rename_selected` (multi-selection) when non-empty.
    /// 2. The single highlighted entry otherwise.
    ///
    /// Directories and binary files are silently skipped; a count of skipped
    /// items is included in the status message when any were omitted.
    pub fn export_context_bundle(&mut self, format: ContextBundleFormat) {
        self.context_bundle_picker_mode = false;

        // Gather the target paths.
        let paths: Vec<std::path::PathBuf> = if !self.rename_selected.is_empty() {
            let mut sorted: Vec<usize> = self.rename_selected.iter().copied().collect();
            sorted.sort_unstable();
            sorted
                .into_iter()
                .filter_map(|i| self.entries.get(i))
                .filter(|e| !e.is_dir)
                .map(|e| e.path.clone())
                .collect()
        } else if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.status_message = Some("[context] No files selected".to_string());
                return;
            }
            vec![entry.path.clone()]
        } else {
            self.status_message = Some("[context] No files selected".to_string());
            return;
        };

        if paths.is_empty() {
            self.status_message = Some("[context] No files selected".to_string());
            return;
        }

        // Build the bundle string.
        let mut bundle = String::new();
        let mut included = 0usize;
        let mut skipped = 0usize;

        match format {
            ContextBundleFormat::PathsOnly => {
                for path in &paths {
                    let rel = self.relative_path(path);
                    bundle.push_str(&rel);
                    bundle.push('\n');
                    included += 1;
                }
            }

            ContextBundleFormat::PathsAndContents => {
                bundle.push_str(&format!("## Context Bundle · {} files\n\n", paths.len()));
                for path in &paths {
                    let rel = self.relative_path(path);
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let tag = lang_tag(&ext);

                    let content_lines = App::read_file_preview(path);
                    // Skip binary / unreadable entries.
                    if content_lines.len() == 1
                        && (content_lines[0] == "[binary file]"
                            || content_lines[0] == "[cannot read file]"
                            || content_lines[0] == "[not a regular file]"
                            || content_lines[0] == "[file too large to preview]")
                    {
                        skipped += 1;
                        continue;
                    }

                    bundle.push_str(&format!("### {}\n", rel));
                    bundle.push_str(&format!("```{}\n", tag));
                    for line in &content_lines {
                        bundle.push_str(line);
                        bundle.push('\n');
                    }
                    bundle.push_str("```\n\n");
                    included += 1;
                }
            }

            ContextBundleFormat::PathsAndDiff => {
                bundle.push_str(&format!(
                    "## Context Bundle · {} changed files\n\n",
                    paths.len()
                ));
                for path in &paths {
                    let rel = self.relative_path(path);

                    if let Some(diff) = git_diff_for_path(path) {
                        bundle.push_str(&format!("### {}\n", rel));
                        bundle.push_str("```diff\n");
                        bundle.push_str(&diff);
                        bundle.push_str("```\n\n");
                        included += 1;
                    } else {
                        // No diff: fall back to full contents with a comment.
                        let content_lines = App::read_file_preview(path);
                        if content_lines.len() == 1
                            && (content_lines[0] == "[binary file]"
                                || content_lines[0] == "[cannot read file]"
                                || content_lines[0] == "[not a regular file]"
                                || content_lines[0] == "[file too large to preview]")
                        {
                            skipped += 1;
                            continue;
                        }

                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let tag = lang_tag(&ext);

                        bundle.push_str(&format!("### {}\n", rel));
                        bundle.push_str(&format!("```{}\n", tag));
                        bundle.push_str("// [untracked — showing full file]\n");
                        for line in &content_lines {
                            bundle.push_str(line);
                            bundle.push('\n');
                        }
                        bundle.push_str("```\n\n");
                        included += 1;
                    }
                }
            }
        }

        if included == 0 {
            self.status_message = Some("[context] No files could be included".to_string());
            return;
        }

        let bundle_bytes = bundle.len();
        const MAX_BUNDLE: usize = 512 * 1024;
        if bundle_bytes > MAX_BUNDLE {
            // Store pending and ask for confirmation.
            self.context_bundle_pending = Some(bundle);
            self.context_bundle_confirm_mode = true;
            self.status_message = Some(format!(
                "[context] Bundle is {:.0} KB — press y to copy, n to cancel",
                bundle_bytes as f64 / 1024.0
            ));
            return;
        }

        self.osc52_copy(&bundle);

        let skip_note = if skipped > 0 {
            format!(" ({} skipped)", skipped)
        } else {
            String::new()
        };
        self.status_message = Some(format!(
            "[context] Copied {} file{} to clipboard{}",
            included,
            if included == 1 { "" } else { "s" },
            skip_note
        ));
    }

    /// Confirm copying the oversized pending bundle.
    pub fn confirm_context_bundle(&mut self) {
        self.context_bundle_confirm_mode = false;
        if let Some(bundle) = self.context_bundle_pending.take() {
            self.osc52_copy(&bundle);
            self.status_message = Some("[context] Copied to clipboard".to_string());
        }
    }

    /// Cancel the oversized bundle copy.
    pub fn cancel_context_bundle_confirm(&mut self) {
        self.context_bundle_confirm_mode = false;
        self.context_bundle_pending = None;
        self.status_message = Some("[context] Cancelled".to_string());
    }

    /// Compute a relative path string from the current working directory.
    fn relative_path(&self, path: &Path) -> String {
        let rel = path.strip_prefix(&self.cwd).unwrap_or(path);
        rel.display().to_string()
    }
}
