//! Recursive filename search (Ctrl+P).
//!
//! Prefers `fd` when it is available on PATH; falls back to a built-in
//! directory walker when `fd` is not installed.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum number of find results returned to the UI.
pub const MAX_FIND_RESULTS: usize = 500;

/// One hit from a recursive filename search.
#[derive(Debug, Clone)]
pub struct FindResult {
    /// Path relative to the search root (used for display).
    pub relative: PathBuf,
    /// Absolute path (used for navigation).
    pub absolute: PathBuf,
}

/// Run a recursive case-insensitive filename search under `root` for files
/// whose name contains `query`.  Returns up to [`MAX_FIND_RESULTS`] results
/// sorted by relevance: exact name match first, then prefix, then substring.
///
/// Returns an empty `Vec` immediately when `query` is empty.
pub fn run_find(query: &str, root: &Path) -> Result<Vec<FindResult>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = if let Some(r) = run_fd(query, root) {
        r
    } else {
        run_walker(query, root)
    };

    sort_results(query, &mut results);
    results.truncate(MAX_FIND_RESULTS);
    Ok(results)
}

// ── fd integration ────────────────────────────────────────────────────────────

/// Try to run `fd` for a fast search. Returns `None` when `fd` is not found.
fn run_fd(query: &str, root: &Path) -> Option<Vec<FindResult>> {
    let output = Command::new("fd")
        .args([
            "--type",
            "f",
            "--max-results",
            "500",
            "--ignore-case",
            query,
        ])
        .current_dir(root)
        .output()
        .ok()?;

    // fd exits non-zero when nothing matches; that is still a valid result.
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.is_empty() && !output.status.success() {
        return None;
    }

    Some(parse_fd_output(&stdout, root))
}

/// Convert newline-delimited `fd` output into [`FindResult`] structs.
///
/// Each line is treated as a path relative to `root`.
pub fn parse_fd_output(output: &str, root: &Path) -> Vec<FindResult> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(MAX_FIND_RESULTS)
        .map(|line| {
            let relative = PathBuf::from(line.trim());
            let absolute = root.join(&relative);
            FindResult { relative, absolute }
        })
        .collect()
}

// ── built-in walker ───────────────────────────────────────────────────────────

/// Built-in recursive walker used as a fallback when `fd` is unavailable.
fn run_walker(query: &str, root: &Path) -> Vec<FindResult> {
    let mut results = Vec::new();
    walk_dir(root, root, query, &mut results);
    results
}

fn walk_dir(root: &Path, dir: &Path, query: &str, results: &mut Vec<FindResult>) {
    if results.len() >= MAX_FIND_RESULTS {
        return;
    }

    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in read.flatten() {
        if results.len() >= MAX_FIND_RESULTS {
            return;
        }

        let path = entry.path();
        let raw_name = entry.file_name();
        let name_str = raw_name.to_string_lossy();

        // Skip hidden entries and well-known large/irrelevant directories.
        if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
            continue;
        }

        if path.is_dir() {
            walk_dir(root, &path, query, results);
        } else {
            let query_lower = query.to_lowercase();
            let name_lower = name_str.to_lowercase();
            if name_lower.contains(&query_lower) {
                if let Ok(relative) = path.strip_prefix(root) {
                    results.push(FindResult {
                        relative: relative.to_path_buf(),
                        absolute: path,
                    });
                }
            }
        }
    }
}

// ── sorting ───────────────────────────────────────────────────────────────────

/// Sort results by relevance.
///
/// Priority (checked against both full filename and bare stem):
///   0 — exact match on full filename or stem
///   1 — prefix match on full filename or stem
///   2 — substring match (fallback)
fn sort_results(query: &str, results: &mut [FindResult]) {
    let q = query.to_lowercase();
    results.sort_by_key(|r| {
        let name = r
            .relative
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = r
            .relative
            .file_stem()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name == q || stem == q {
            0u8
        } else if name.starts_with(&q) || stem.starts_with(&q) {
            1
        } else {
            2
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_result(relative: &str, root: &Path) -> FindResult {
        let rel = PathBuf::from(relative);
        let abs = root.join(&rel);
        FindResult {
            relative: rel,
            absolute: abs,
        }
    }

    /// Given: `fd` output with 3 relative paths
    /// When: parse_fd_output is called
    /// Then: 3 FindResult entries are returned with matching relative paths
    #[test]
    fn parse_fd_output_three_lines() {
        let root = Path::new("/tmp");
        let output = "src/main.rs\nsrc/lib.rs\ntests/foo.rs\n";
        let results = parse_fd_output(output, root);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].relative, PathBuf::from("src/main.rs"));
        assert_eq!(results[1].relative, PathBuf::from("src/lib.rs"));
        assert_eq!(results[2].relative, PathBuf::from("tests/foo.rs"));
    }

    /// Given: empty fd output
    /// When: parse_fd_output is called
    /// Then: empty vec is returned
    #[test]
    fn parse_fd_output_empty_string_returns_empty() {
        let root = Path::new("/tmp");
        let results = parse_fd_output("", root);
        assert!(results.is_empty());
    }

    /// Given: fd output with more than MAX_FIND_RESULTS lines
    /// When: parse_fd_output is called
    /// Then: exactly MAX_FIND_RESULTS results are returned
    #[test]
    fn parse_fd_output_truncates_at_max() {
        let root = Path::new("/tmp");
        let output: String = (0..600).map(|i| format!("file{i}.rs\n")).collect();
        let results = parse_fd_output(&output, root);
        assert_eq!(results.len(), MAX_FIND_RESULTS);
    }

    /// Given: results with exact, prefix, and substring matches for "main"
    /// When: sort_results is called
    /// Then: exact comes first, prefix second, substring last
    #[test]
    fn sort_results_orders_exact_prefix_substring() {
        let root = Path::new("/tmp");
        let mut results = vec![
            make_result("src/not_main_thing.rs", root),
            make_result("src/main_loop.rs", root),
            make_result("src/main.rs", root),
        ];
        sort_results("main", &mut results);
        assert_eq!(results[0].relative.file_name().unwrap(), "main.rs");
        assert_eq!(results[1].relative.file_name().unwrap(), "main_loop.rs");
        assert_eq!(
            results[2].relative.file_name().unwrap(),
            "not_main_thing.rs"
        );
    }

    /// Given: an empty query string
    /// When: run_find is called
    /// Then: returns Ok with an empty vec without touching the filesystem
    #[test]
    fn run_find_empty_query_returns_empty() {
        let root = std::env::temp_dir();
        let results = run_find("", &root).unwrap();
        assert!(results.is_empty());
    }

    /// Given: a temp directory containing a uniquely named file
    /// When: run_walker is called with a query matching that filename
    /// Then: the result contains the file's absolute path
    #[test]
    fn walker_finds_file_in_temp_dir() {
        let root = std::env::temp_dir().join(format!("trek_walker_root_{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let fname = format!("trek_walker_test_{}.txt", std::process::id());
        let fpath = root.join(&fname);
        fs::write(&fpath, b"hello").unwrap();

        let results = run_walker("trek_walker_test", &root);
        let _ = fs::remove_file(&fpath);
        let _ = fs::remove_dir(&root);

        assert!(
            results.iter().any(|r| r.absolute == fpath),
            "expected to find {fname} in results: {results:?}",
        );
    }

    /// Given: a temp directory with a hidden subdirectory containing a file
    /// When: run_walker is called with a query matching that file's name
    /// Then: the hidden directory's contents are not returned
    #[test]
    fn walker_skips_hidden_directories() {
        let root = std::env::temp_dir().join(format!("trek_walker_root_{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let hidden_dir = root.join(format!(".trek_hidden_{}", std::process::id()));
        let _ = fs::create_dir_all(&hidden_dir);
        let fname = format!("trek_hidden_file_{}.txt", std::process::id());
        let hidden_file = hidden_dir.join(&fname);
        let _ = fs::write(&hidden_file, b"hidden");

        let results = run_walker(&fname, &root);

        let _ = fs::remove_file(&hidden_file);
        let _ = fs::remove_dir(&hidden_dir);
        let _ = fs::remove_dir(&root);

        assert!(
            !results.iter().any(|r| r.absolute == hidden_file),
            "walker must not descend into hidden directories"
        );
    }
}
