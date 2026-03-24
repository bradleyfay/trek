use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

const PREVIEW_THEME: &str = "base16-ocean.dark";

/// Owns the syntax/theme data loaded once at startup.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    /// Load bundled syntax definitions and themes. Call once at startup.
    pub fn new() -> Self {
        // Stub — does NOT load real data. Tests will fail.
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight `lines` for the given file extension.
    ///
    /// Returns `None` if the extension is unrecognized (caller renders plain text).
    /// Processes at most `max_lines` lines to keep render time bounded.
    pub fn highlight(
        &self,
        lines: &[String],
        extension: &str,
        max_lines: usize,
    ) -> Option<Vec<Line<'static>>> {
        let syntax = self.syntax_set.find_syntax_by_extension(extension)?;
        let theme = self.theme_set.themes.get(PREVIEW_THEME)?;
        let mut h = HighlightLines::new(syntax, theme);

        let mut result = Vec::with_capacity(lines.len().min(max_lines));
        for line_str in lines.iter().take(max_lines) {
            // syntect expects a trailing newline for correct tokenization.
            let with_newline = format!("{}\n", line_str);
            let regions = match h.highlight_line(&with_newline, &self.syntax_set) {
                Ok(r) => r,
                Err(_) => return None,
            };

            let spans: Vec<Span<'static>> = regions
                .iter()
                .map(|(style, text)| {
                    let owned = text.trim_end_matches('\n').to_string();
                    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    Span::styled(owned, Style::default().fg(fg))
                })
                .filter(|s| !s.content.is_empty())
                .collect();

            result.push(Line::from(spans));
        }

        Some(result)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn hl() -> Highlighter {
        Highlighter::new()
    }

    /// Given: a recognized extension (.rs) with valid Rust lines
    /// When: highlight is called
    /// Then: Some(lines) is returned with the same line count
    #[test]
    fn highlight_rust_returns_some() {
        let h = hl();
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"hello\");".to_string(),
            "}".to_string(),
        ];
        let result = h.highlight(&lines, "rs", 100);
        assert!(result.is_some(), "expected Some for .rs extension");
        assert_eq!(result.unwrap().len(), 3);
    }

    /// Given: an unrecognized extension (.xyz)
    /// When: highlight is called
    /// Then: None is returned so the caller renders plain text
    #[test]
    fn highlight_unknown_extension_returns_none() {
        let h = hl();
        let lines = vec!["some text".to_string()];
        assert!(
            h.highlight(&lines, "xyz", 100).is_none(),
            "expected None for unrecognized extension"
        );
    }

    /// Given: 10 lines and max_lines = 2
    /// When: highlight is called with a recognized extension
    /// Then: at most 2 lines are in the result
    #[test]
    fn highlight_caps_at_max_lines() {
        let h = hl();
        let lines: Vec<String> = (0..10).map(|i| format!("// line {}", i)).collect();
        let result = h.highlight(&lines, "rs", 2).unwrap();
        assert_eq!(result.len(), 2);
    }

    /// Given: an empty slice for a recognized extension
    /// When: highlight is called
    /// Then: Some([]) is returned (not None)
    #[test]
    fn highlight_empty_returns_some_empty() {
        let h = hl();
        let result = h.highlight(&[], "rs", 100).unwrap();
        assert!(result.is_empty());
    }

    /// Given: a .py extension
    /// When: highlight is called
    /// Then: Some is returned (Python is a bundled syntax)
    #[test]
    fn highlight_python_returns_some() {
        let h = hl();
        let lines = vec!["def hello():".to_string(), "    pass".to_string()];
        assert!(h.highlight(&lines, "py", 100).is_some());
    }

    /// Given: a .yaml extension
    /// When: highlight is called
    /// Then: Some is returned (YAML is a bundled syntax)
    #[test]
    fn highlight_yaml_returns_some() {
        let h = hl();
        let lines = vec!["key: value".to_string(), "list:".to_string()];
        assert!(h.highlight(&lines, "yaml", 100).is_some());
    }
}
