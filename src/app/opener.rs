/// Rifle-style configurable file opener rules.
///
/// Trek loads opener rules from `$XDG_CONFIG_HOME/trek/opener.conf`
/// (falling back to `~/.config/trek/opener.conf`). Rules are evaluated
/// top-to-bottom; the first matching rule wins. When no config file is found,
/// built-in defaults replicate the prior hardcoded routing behaviour.
///
/// # Config format
///
/// ```text
/// # Lines beginning with # are comments. Blank lines are ignored.
/// # Format:  <matcher> <pattern> : <command>
/// #
/// # Matchers:
/// #   ext <ext1|ext2|...>   — file extension match (case-insensitive)
/// #   glob <pattern>        — shell glob match against the filename
/// #
/// # Use {} as the placeholder for the file path in the command.
/// # The command is executed via the system shell (sh -c).
///
/// ext md|markdown : cmux markdown open {}
/// ext html|htm    : open {}
/// glob *          : $EDITOR {}
/// ```
use std::path::Path;

/// A single opener rule mapping a file pattern to a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenerRule {
    pub matcher: Matcher,
    /// Command template.  `{}` is replaced with the quoted file path.
    pub command: String,
}

/// The pattern half of an opener rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// Match by file extension (case-insensitive). Multiple extensions
    /// separated by `|` in the config are stored as individual strings.
    Ext(Vec<String>),
    /// Shell glob matched against the file **name** (not the full path).
    Glob(String),
}

impl Matcher {
    /// Returns `true` when this matcher applies to `path`.
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Matcher::Ext(exts) => {
                let file_ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                exts.iter().any(|e| e == &file_ext)
            }
            Matcher::Glob(pattern) => {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                glob_match(pattern, filename)
            }
        }
    }
}

/// Minimal glob implementation supporting `*` (any sequence) and `?` (one char).
///
/// Matching is against the full pattern — the pattern must cover the entire
/// filename, not just a substring. This is sufficient for the common cases
/// (`*`, `*.rs`, `Makefile`).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_match_inner(&p, &t)
}

fn glob_match_inner(p: &[char], t: &[char]) -> bool {
    match (p, t) {
        ([], []) => true,
        ([], _) => false,
        (['*', rest @ ..], _) => {
            // `*` matches zero or more characters.
            if glob_match_inner(rest, t) {
                return true;
            }
            for i in 0..t.len() {
                if glob_match_inner(rest, &t[i + 1..]) {
                    return true;
                }
            }
            false
        }
        (['?', p_rest @ ..], [_, t_rest @ ..]) => glob_match_inner(p_rest, t_rest),
        (['?', ..], []) => false,
        ([pc, p_rest @ ..], [tc, t_rest @ ..]) if pc == tc => glob_match_inner(p_rest, t_rest),
        _ => false,
    }
}

/// The loaded opener configuration.
#[derive(Debug, Clone, Default)]
pub struct OpenerConfig {
    pub rules: Vec<OpenerRule>,
}

impl OpenerConfig {
    /// Load the opener config from the standard config path.
    ///
    /// Returns `None` when the config file does not exist. Returns an empty
    /// config (not `None`) when the file exists but contains only comments or
    /// blank lines.
    pub fn load() -> Option<Self> {
        let path = config_path()?;
        let text = std::fs::read_to_string(&path).ok()?;
        Some(Self::parse(&text))
    }

    /// Parse a config file body into an `OpenerConfig`.
    ///
    /// Lines that cannot be parsed are silently skipped so that a single bad
    /// rule does not break all file opens.
    pub fn parse(text: &str) -> Self {
        let rules = text.lines().filter_map(parse_line).collect();
        Self { rules }
    }

    /// Find the first rule matching `path` and return its command template.
    ///
    /// Returns `None` when no rule matches.
    pub fn find_command(&self, path: &Path) -> Option<&str> {
        self.rules
            .iter()
            .find(|r| r.matcher.matches(path))
            .map(|r| r.command.as_str())
    }

    /// Expand a command template by substituting `{}` with the shell-escaped
    /// file path.
    ///
    /// If the template does not contain `{}`, the path is appended with a
    /// space separator.
    pub fn expand_command(template: &str, path: &Path) -> String {
        let path_str = path.to_string_lossy();
        let escaped = shell_escape_path(&path_str);
        if template.contains("{}") {
            template.replace("{}", &escaped)
        } else {
            format!("{} {}", template, escaped)
        }
    }
}

/// Built-in default opener rules used when no config file is present.
///
/// - Markdown → cmux markdown viewer (`cmux markdown open`)
/// - HTML → cmux embedded browser (`cmux browser open`)
/// - Binary/document types → system default opener (`open` / `xdg-open`)
/// - Everything else → `$EDITOR`
pub fn default_rules() -> Vec<OpenerRule> {
    vec![
        OpenerRule {
            matcher: Matcher::Ext(vec!["md".into(), "markdown".into()]),
            command: "cmux markdown open {}".into(),
        },
        OpenerRule {
            matcher: Matcher::Ext(vec!["html".into(), "htm".into()]),
            command: "cmux browser open {}".into(),
        },
        OpenerRule {
            matcher: Matcher::Ext(vec![
                "pdf".into(),
                "png".into(),
                "jpg".into(),
                "jpeg".into(),
                "gif".into(),
                "svg".into(),
                "webp".into(),
                "ico".into(),
                "bmp".into(),
                "tiff".into(),
                "tif".into(),
            ]),
            command: system_open_command().to_string(),
        },
        OpenerRule {
            matcher: Matcher::Glob("*".into()),
            command: "$EDITOR {}".into(),
        },
    ]
}

/// Returns the platform-appropriate system-open command with `{}` placeholder.
pub fn system_open_command() -> &'static str {
    #[cfg(target_os = "macos")]
    return "open {}";
    #[cfg(not(target_os = "macos"))]
    return "xdg-open {}";
}

/// Resolve the opener config file path, respecting `$XDG_CONFIG_HOME`.
fn config_path() -> Option<std::path::PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(dirs_config_home)?;
    Some(base.join("trek").join("opener.conf"))
}

/// Returns `~/.config` as a `PathBuf`, or `None` if `$HOME` is unset.
fn dirs_config_home() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config"))
}

/// Parse a single config line into an `OpenerRule`.
///
/// Expected formats:
/// - `ext <exts> : <command>`
/// - `glob <pattern> : <command>`
///
/// Returns `None` for blank lines and comment lines (starting with `#`).
fn parse_line(line: &str) -> Option<OpenerRule> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (lhs, command) = trimmed.split_once(':')?;
    let command = command.trim().to_string();
    if command.is_empty() {
        return None;
    }

    let mut parts = lhs.split_whitespace();
    let kind = parts.next()?;
    let pattern = parts.next()?;

    match kind {
        "ext" => {
            let exts: Vec<String> = pattern
                .split('|')
                .map(|e| e.trim().to_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
            if exts.is_empty() {
                return None;
            }
            Some(OpenerRule {
                matcher: Matcher::Ext(exts),
                command,
            })
        }
        "glob" => Some(OpenerRule {
            matcher: Matcher::Glob(pattern.to_string()),
            command,
        }),
        _ => None,
    }
}

/// Shell-escape a path for safe interpolation into a command string.
///
/// Wraps in single quotes and escapes any embedded single quotes.
fn shell_escape_path(s: &str) -> String {
    if s.chars()
        .any(|c| " '\"\t\n\\()[]{};&|<>*?$`!#~".contains(c))
    {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Matcher::Ext ──────────────────────────────────────────────────────────

    /// Given: an Ext matcher for "rs"
    /// When: path with extension "rs" is tested
    /// Then: matches returns true
    #[test]
    fn ext_matcher_matches_correct_extension() {
        let m = Matcher::Ext(vec!["rs".into()]);
        assert!(m.matches(Path::new("main.rs")));
    }

    /// Given: an Ext matcher for "rs"
    /// When: path with extension "md" is tested
    /// Then: matches returns false
    #[test]
    fn ext_matcher_rejects_wrong_extension() {
        let m = Matcher::Ext(vec!["rs".into()]);
        assert!(!m.matches(Path::new("README.md")));
    }

    /// Given: an Ext matcher with multiple extensions
    /// When: each extension is tested
    /// Then: all match
    #[test]
    fn ext_matcher_matches_any_of_multiple_extensions() {
        let m = Matcher::Ext(vec!["html".into(), "htm".into()]);
        assert!(m.matches(Path::new("index.html")));
        assert!(m.matches(Path::new("page.htm")));
        assert!(!m.matches(Path::new("script.js")));
    }

    /// Given: an Ext matcher for lowercase "rs"
    /// When: a path with uppercase extension "RS" is tested
    /// Then: matches (case-insensitive)
    #[test]
    fn ext_matcher_is_case_insensitive() {
        let m = Matcher::Ext(vec!["rs".into()]);
        assert!(m.matches(Path::new("main.RS")));
    }

    /// Given: an Ext matcher for "rs"
    /// When: a path with no extension is tested
    /// Then: does not match
    #[test]
    fn ext_matcher_rejects_no_extension() {
        let m = Matcher::Ext(vec!["rs".into()]);
        assert!(!m.matches(Path::new("Makefile")));
    }

    // ── Matcher::Glob ─────────────────────────────────────────────────────────

    /// Given: a Glob matcher with pattern "*"
    /// When: any filename is tested
    /// Then: always matches
    #[test]
    fn glob_star_matches_everything() {
        let m = Matcher::Glob("*".into());
        assert!(m.matches(Path::new("anything.rs")));
        assert!(m.matches(Path::new("Makefile")));
        assert!(m.matches(Path::new(".hidden")));
    }

    /// Given: a Glob matcher with pattern "*.rs"
    /// When: a Rust file is tested
    /// Then: matches
    #[test]
    fn glob_star_rs_matches_rust_files() {
        let m = Matcher::Glob("*.rs".into());
        assert!(m.matches(Path::new("main.rs")));
        assert!(!m.matches(Path::new("main.py")));
    }

    /// Given: a Glob matcher with pattern "Make*"
    /// When: "Makefile" is tested
    /// Then: matches
    #[test]
    fn glob_prefix_star_matches() {
        let m = Matcher::Glob("Make*".into());
        assert!(m.matches(Path::new("Makefile")));
        assert!(m.matches(Path::new("Makefile.am")));
        assert!(!m.matches(Path::new("GNUmakefile")));
    }

    /// Given: a Glob matcher with "?" wildcard
    /// When: a filename with that single-char slot is tested
    /// Then: matches correctly
    #[test]
    fn glob_question_mark_matches_single_char() {
        let m = Matcher::Glob("file?.txt".into());
        assert!(m.matches(Path::new("file1.txt")));
        assert!(m.matches(Path::new("fileA.txt")));
        assert!(!m.matches(Path::new("file.txt")));
        assert!(!m.matches(Path::new("file12.txt")));
    }

    // ── OpenerConfig::parse ───────────────────────────────────────────────────

    /// Given: a valid config with one ext rule and one glob rule
    /// When: parse is called
    /// Then: two rules are returned in order
    #[test]
    fn parse_returns_rules_in_order() {
        let config = OpenerConfig::parse("ext html|htm : open {}\nglob * : $EDITOR {}");
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].command, "open {}");
        assert_eq!(config.rules[1].command, "$EDITOR {}");
    }

    /// Given: config with comment and blank lines
    /// When: parse is called
    /// Then: comments and blanks are ignored
    #[test]
    fn parse_ignores_comments_and_blanks() {
        let config = OpenerConfig::parse("# comment\n\next rs : code {}\n\n# another comment");
        assert_eq!(config.rules.len(), 1);
    }

    /// Given: config with an unrecognised matcher kind
    /// When: parse is called
    /// Then: that line is silently skipped
    #[test]
    fn parse_skips_unknown_matcher_kind() {
        let config = OpenerConfig::parse("mime text/plain : less {}\next rs : code {}");
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].command, "code {}");
    }

    /// Given: an ext rule with multiple pipe-separated extensions
    /// When: parse is called
    /// Then: all extensions are captured
    #[test]
    fn parse_splits_pipe_separated_extensions() {
        let config = OpenerConfig::parse("ext png|jpg|jpeg : open {}");
        assert_eq!(config.rules.len(), 1);
        if let Matcher::Ext(exts) = &config.rules[0].matcher {
            assert_eq!(exts, &["png", "jpg", "jpeg"]);
        } else {
            panic!("expected Ext matcher");
        }
    }

    // ── OpenerConfig::find_command ────────────────────────────────────────────

    /// Given: a config with an ext rule for "html"
    /// When: find_command is called with an html path
    /// Then: returns the matching command
    #[test]
    fn find_command_returns_matching_rule() {
        let config = OpenerConfig::parse("ext html : open {}\nglob * : $EDITOR {}");
        let cmd = config.find_command(Path::new("index.html"));
        assert_eq!(cmd, Some("open {}"));
    }

    /// Given: a config with an ext rule for "html" followed by a glob "*"
    /// When: find_command is called with a non-html path
    /// Then: returns the glob fallback
    #[test]
    fn find_command_falls_back_to_glob_star() {
        let config = OpenerConfig::parse("ext html : open {}\nglob * : $EDITOR {}");
        let cmd = config.find_command(Path::new("main.rs"));
        assert_eq!(cmd, Some("$EDITOR {}"));
    }

    /// Given: a config where the first matching rule is ext, not glob
    /// When: find_command is called
    /// Then: first-match-wins: ext rule is returned, not the glob
    #[test]
    fn find_command_first_match_wins() {
        let config =
            OpenerConfig::parse("glob * : first {}\next rs : second {}\nglob * : third {}");
        let cmd = config.find_command(Path::new("main.rs"));
        assert_eq!(cmd, Some("first {}"));
    }

    /// Given: an empty config
    /// When: find_command is called
    /// Then: returns None
    #[test]
    fn find_command_returns_none_when_no_rules() {
        let config = OpenerConfig::parse("");
        assert_eq!(config.find_command(Path::new("main.rs")), None);
    }

    // ── OpenerConfig::expand_command ─────────────────────────────────────────

    /// Given: a command template with "{}"
    /// When: expand_command is called with a simple path
    /// Then: "{}" is replaced by the path
    #[test]
    fn expand_command_substitutes_placeholder() {
        let result = OpenerConfig::expand_command("open {}", Path::new("/tmp/file.txt"));
        assert_eq!(result, "open /tmp/file.txt");
    }

    /// Given: a command template with "{}" and a path containing spaces
    /// When: expand_command is called
    /// Then: path is shell-quoted
    #[test]
    fn expand_command_quotes_paths_with_spaces() {
        let result = OpenerConfig::expand_command("open {}", Path::new("/home/user/my file.txt"));
        assert_eq!(result, "open '/home/user/my file.txt'");
    }

    /// Given: a command template without "{}"
    /// When: expand_command is called
    /// Then: path is appended with a space
    #[test]
    fn expand_command_appends_path_when_no_placeholder() {
        let result = OpenerConfig::expand_command("open", Path::new("/tmp/file.txt"));
        assert_eq!(result, "open /tmp/file.txt");
    }

    // ── default_rules ─────────────────────────────────────────────────────────

    /// Given: the default rules
    /// When: find_command is called with a Markdown file
    /// Then: returns the cmux markdown viewer command
    #[test]
    fn default_rules_route_markdown_to_cmux_viewer() {
        let config = OpenerConfig {
            rules: default_rules(),
        };
        assert_eq!(
            config.find_command(Path::new("README.md")),
            Some("cmux markdown open {}")
        );
        assert_eq!(
            config.find_command(Path::new("notes.markdown")),
            Some("cmux markdown open {}")
        );
    }

    /// Given: the default rules
    /// When: find_command is called with an HTML file
    /// Then: returns the cmux browser command
    #[test]
    fn default_rules_route_html_to_cmux_browser() {
        let config = OpenerConfig {
            rules: default_rules(),
        };
        assert_eq!(
            config.find_command(Path::new("index.html")),
            Some("cmux browser open {}")
        );
        assert_eq!(
            config.find_command(Path::new("page.htm")),
            Some("cmux browser open {}")
        );
    }

    /// Given: the default rules
    /// When: find_command is called with a Rust source file
    /// Then: returns the $EDITOR fallback
    #[test]
    fn default_rules_route_code_to_editor() {
        let config = OpenerConfig {
            rules: default_rules(),
        };
        let cmd = config.find_command(Path::new("main.rs")).unwrap_or("");
        assert_eq!(cmd, "$EDITOR {}");
    }

    /// Given: the default rules
    /// When: find_command is called with a PNG image
    /// Then: returns a system-open command
    #[test]
    fn default_rules_route_image_to_system_open() {
        let config = OpenerConfig {
            rules: default_rules(),
        };
        let cmd = config.find_command(Path::new("photo.png")).unwrap_or("");
        assert!(
            cmd.starts_with("open") || cmd.starts_with("xdg-open"),
            "expected system open command, got: {}",
            cmd
        );
    }

    // ── shell_escape_path ─────────────────────────────────────────────────────

    /// Given: a path with no special characters
    /// When: shell_escape_path is called
    /// Then: path is returned unchanged
    #[test]
    fn shell_escape_path_leaves_simple_paths_unchanged() {
        assert_eq!(
            shell_escape_path("/home/user/file.txt"),
            "/home/user/file.txt"
        );
    }

    /// Given: a path with a space
    /// When: shell_escape_path is called
    /// Then: path is wrapped in single quotes
    #[test]
    fn shell_escape_path_quotes_paths_with_spaces() {
        assert_eq!(
            shell_escape_path("/home/user/my file.txt"),
            "'/home/user/my file.txt'"
        );
    }

    /// Given: a path with a single quote
    /// When: shell_escape_path is called
    /// Then: single quote is escaped within the quoting
    #[test]
    fn shell_escape_path_handles_single_quote() {
        assert_eq!(
            shell_escape_path("/home/user/it's"),
            "'/home/user/it'\\''s'"
        );
    }

    // ── OpenerConfig::load from file ──────────────────────────────────────────

    /// Given: a temporary config file with valid rules
    /// When: OpenerConfig::parse is called with its contents
    /// Then: rules are loaded correctly
    #[test]
    fn parse_loads_config_from_text() {
        let text = "# opener config\next md : cmux markdown open {}\nglob * : $EDITOR {}";
        let config = OpenerConfig::parse(text);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].matcher, Matcher::Ext(vec!["md".into()]));
        assert_eq!(config.rules[0].command, "cmux markdown open {}");
    }
}
