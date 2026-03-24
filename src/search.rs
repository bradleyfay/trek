use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum total match entries returned by a single search.
pub const MAX_RESULTS: usize = 500;

/// A single matching line within a file.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub line_number: u64,
    pub line_content: String,
}

/// All matches within a single file.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResultGroup {
    pub file: PathBuf,
    pub matches: Vec<SearchResult>,
}

/// Run `rg --line-number --color never --with-filename <query> <dir>` and
/// return grouped results capped at `MAX_RESULTS`.
///
/// Returns `Err` if `rg` is not found in `PATH` or the query itself is an
/// invalid regex (rg exits 2 with a message on stderr).
pub fn run_rg(query: &str, dir: &Path) -> Result<Vec<SearchResultGroup>, String> {
    let out = Command::new("rg")
        .args(["--line-number", "--color", "never", "--with-filename"])
        .arg(query)
        .arg(dir)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "content search requires ripgrep (rg) — not found in PATH".to_string()
            } else {
                format!("rg error: {e}")
            }
        })?;

    // rg exit codes: 0 = matches found, 1 = no matches, 2 = error.
    match out.status.code() {
        Some(0) | Some(1) => {}
        _ => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !err.is_empty() {
                return Err(err);
            }
        }
    }

    Ok(parse_rg_output(&out.stdout, dir))
}

/// Parse rg output lines of the form `<path>:<line_number>:<content>`.
///
/// Groups consecutive matches by file path. Caps total entries at `MAX_RESULTS`.
/// Paths are stripped of the `base` prefix for display.
pub fn parse_rg_output(output: &[u8], base: &Path) -> Vec<SearchResultGroup> {
    let text = String::from_utf8_lossy(output);
    let mut groups: Vec<SearchResultGroup> = Vec::new();
    let mut total = 0usize;

    for raw_line in text.lines() {
        if total >= MAX_RESULTS {
            break;
        }
        // Format: `<path>:<line_number>:<content>`
        // Split on the first two colons only — content may itself contain colons.
        let mut parts = raw_line.splitn(3, ':');
        let path_str = match parts.next() {
            Some(p) => p,
            None => continue,
        };
        let line_num_str = match parts.next() {
            Some(n) => n,
            None => continue,
        };
        let content = parts.next().unwrap_or("").to_string();

        let line_number: u64 = match line_num_str.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let file = PathBuf::from(path_str);
        let display_file = if let Ok(rel) = file.strip_prefix(base) {
            rel.to_path_buf()
        } else {
            file
        };

        let result = SearchResult {
            line_number,
            line_content: content,
        };

        match groups.last_mut() {
            Some(g) if g.file == display_file => {
                g.matches.push(result);
            }
            _ => {
                groups.push(SearchResultGroup {
                    file: display_file,
                    matches: vec![result],
                });
            }
        }
        total += 1;
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PathBuf {
        PathBuf::from("/project")
    }

    fn mk_line(file: &str, lineno: u64, content: &str) -> String {
        format!("{file}:{lineno}:{content}")
    }

    /// Given: valid rg output with two files
    /// When: parse_rg_output is called
    /// Then: results are grouped by file
    #[test]
    fn parse_groups_by_file() {
        let input = [
            mk_line("/project/src/main.rs", 10, "let x = foo();"),
            mk_line("/project/src/main.rs", 20, "bar(foo)"),
            mk_line("/project/src/lib.rs", 5, "pub fn foo()"),
        ]
        .join("\n");
        let groups = parse_rg_output(input.as_bytes(), &base());
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].file, PathBuf::from("src/main.rs"));
        assert_eq!(groups[0].matches.len(), 2);
        assert_eq!(groups[1].file, PathBuf::from("src/lib.rs"));
        assert_eq!(groups[1].matches.len(), 1);
    }

    /// Given: empty rg output
    /// When: parse_rg_output is called
    /// Then: returns an empty vec
    #[test]
    fn parse_empty_output_returns_empty() {
        let groups = parse_rg_output(b"", &base());
        assert!(groups.is_empty());
    }

    /// Given: output where a line's content contains colons
    /// When: parse_rg_output is called
    /// Then: the colon is preserved in the content, not treated as a delimiter
    #[test]
    fn parse_preserves_colons_in_content() {
        let input = "/project/src/a.rs:42:http://example.com\n";
        let groups = parse_rg_output(input.as_bytes(), &base());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].matches[0].line_content, "http://example.com");
        assert_eq!(groups[0].matches[0].line_number, 42);
    }

    /// Given: output with more than MAX_RESULTS matches
    /// When: parse_rg_output is called
    /// Then: at most MAX_RESULTS results are returned
    #[test]
    fn parse_caps_at_max_results() {
        let lines: Vec<String> = (1..=(MAX_RESULTS + 10) as u64)
            .map(|n| mk_line("/project/src/big.rs", n, "match"))
            .collect();
        let input = lines.join("\n");
        let groups = parse_rg_output(input.as_bytes(), &base());
        let total: usize = groups.iter().map(|g| g.matches.len()).sum();
        assert_eq!(total, MAX_RESULTS);
    }

    /// Given: a malformed line (missing second colon)
    /// When: parse_rg_output is called
    /// Then: the line is skipped without panic
    #[test]
    fn parse_skips_malformed_lines() {
        let input = "this_line_has_no_colon\n/project/a.rs:1:ok\n";
        let groups = parse_rg_output(input.as_bytes(), &base());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].matches.len(), 1);
    }

    /// Given: a line where line_number field is not a valid integer
    /// When: parse_rg_output is called
    /// Then: the line is skipped without panic
    #[test]
    fn parse_skips_non_numeric_line_number() {
        let input = "/project/a.rs:not_a_number:content\n/project/a.rs:5:ok\n";
        let groups = parse_rg_output(input.as_bytes(), &base());
        assert_eq!(groups[0].matches.len(), 1);
        assert_eq!(groups[0].matches[0].line_number, 5);
    }
}
