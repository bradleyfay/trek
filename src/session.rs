//! Session persistence — save and restore cwd, marks, and view settings between Trek invocations.
//!
//! Stored at `$XDG_DATA_HOME/trek/session` (fallback: `~/.local/share/trek/session`).
//! Simple `key=value` format; no external dependencies.

use crate::app::{SortMode, SortOrder};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub struct Session {
    /// Restored working directory (`None` if file missing or dir deleted).
    pub cwd: Option<PathBuf>,
    /// Restored mark slots; only entries where the path still exists are included.
    pub marks: HashMap<char, PathBuf>,
    /// Whether hidden files were visible when Trek last exited.
    pub show_hidden: bool,
    /// Sort field in use when Trek last exited.
    pub sort_mode: SortMode,
    /// Sort direction in use when Trek last exited.
    pub sort_order: SortOrder,
    /// Name of the selected entry when Trek last exited (not an index — indices shift).
    pub selected_name: Option<String>,
}

/// Return the path of the session file.
pub fn session_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/share")
        });
    base.join("trek").join("session")
}

/// Load the session file. Returns a default `Session` if the file is absent
/// or unreadable — never panics.
pub fn load() -> Session {
    load_from(&session_path())
}

/// Load session state from an explicit file path.
///
/// Identical to `load()` but accepts an explicit path rather than reading from
/// the environment, making it usable in tests without mutating env vars.
pub(crate) fn load_from(path: &Path) -> Session {
    let Ok(file) = std::fs::File::open(path) else {
        return Session {
            cwd: None,
            marks: HashMap::new(),
            show_hidden: false,
            sort_mode: SortMode::default(),
            sort_order: SortOrder::default(),
            selected_name: None,
        };
    };
    let mut cwd = None;
    let mut marks = HashMap::new();
    let mut show_hidden = false;
    let mut sort_mode = SortMode::default();
    let mut sort_order = SortOrder::default();
    let mut selected_name = None;

    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let line = line.trim().to_owned();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        match key {
            "cwd" => {
                let path = PathBuf::from(val);
                if path.is_dir() {
                    cwd = Some(path);
                }
            }
            "show_hidden" => show_hidden = val == "true",
            "sort_mode" => sort_mode = parse_sort_mode(val),
            "sort_order" => sort_order = parse_sort_order(val),
            "selected" => {
                if !val.is_empty() {
                    selected_name = Some(val.to_owned());
                }
            }
            _ => {
                if let Some(letter) = key.strip_prefix("mark.") {
                    if let Some(c) = letter.chars().next().filter(|ch| ch.is_alphabetic()) {
                        let path = PathBuf::from(val);
                        if path.exists() {
                            marks.insert(c, path);
                        }
                    }
                }
            }
        }
    }
    Session {
        cwd,
        marks,
        show_hidden,
        sort_mode,
        sort_order,
        selected_name,
    }
}

/// Write session state to disk. Errors are silently ignored at call sites —
/// a failed save must never crash Trek or block a clean exit.
pub fn save(
    cwd: &Path,
    marks: &HashMap<char, PathBuf>,
    show_hidden: bool,
    sort_mode: SortMode,
    sort_order: SortOrder,
    selected_name: Option<&str>,
) -> std::io::Result<()> {
    save_to(
        &session_path(),
        cwd,
        marks,
        show_hidden,
        sort_mode,
        sort_order,
        selected_name,
    )
}

/// Write session state to an explicit file path.
///
/// Identical to `save()` but accepts an explicit path rather than reading from
/// the environment, making it usable in tests without mutating env vars.
pub(crate) fn save_to(
    path: &Path,
    cwd: &Path,
    marks: &HashMap<char, PathBuf>,
    show_hidden: bool,
    sort_mode: SortMode,
    sort_order: SortOrder,
    selected_name: Option<&str>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "cwd={}", cwd.display())?;
    writeln!(f, "show_hidden={}", show_hidden)?;
    writeln!(f, "sort_mode={}", format_sort_mode(sort_mode))?;
    writeln!(f, "sort_order={}", format_sort_order(sort_order))?;
    writeln!(f, "selected={}", selected_name.unwrap_or(""))?;
    let mut sorted: Vec<_> = marks.iter().collect();
    sorted.sort_by_key(|(c, _)| *c);
    for (c, p) in sorted {
        writeln!(f, "mark.{}={}", c, p.display())?;
    }
    Ok(())
}

fn parse_sort_mode(s: &str) -> SortMode {
    match s {
        "size" => SortMode::Size,
        "modified" => SortMode::Modified,
        "extension" => SortMode::Extension,
        _ => SortMode::Name,
    }
}

fn parse_sort_order(s: &str) -> SortOrder {
    if s == "descending" {
        SortOrder::Descending
    } else {
        SortOrder::Ascending
    }
}

fn format_sort_mode(m: SortMode) -> &'static str {
    match m {
        SortMode::Name => "name",
        SortMode::Size => "size",
        SortMode::Modified => "modified",
        SortMode::Extension => "extension",
    }
}

fn format_sort_order(o: SortOrder) -> &'static str {
    match o {
        SortOrder::Ascending => "ascending",
        SortOrder::Descending => "descending",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Return a unique per-test temporary session file path. No env mutation needed.
    fn temp_session_path(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("trek_sess_{}_{}_{}", std::process::id(), n, tag))
            .join("trek")
            .join("session")
    }

    /// Helper: save with all fields set to their defaults.
    fn save_defaults(session_file: &Path, cwd: &Path) {
        save_to(
            session_file,
            cwd,
            &HashMap::new(),
            false,
            SortMode::default(),
            SortOrder::default(),
            None,
        )
        .unwrap();
    }

    /// Given: no session file exists
    /// When: load_from() is called
    /// Then: returns empty session without panicking
    #[test]
    fn load_returns_empty_session_when_no_file() {
        let path = temp_session_path("empty");
        let s = load_from(&path);
        assert!(s.cwd.is_none());
        assert!(s.marks.is_empty());
        assert!(!s.show_hidden);
        assert_eq!(s.sort_mode, SortMode::Name);
        assert_eq!(s.sort_order, SortOrder::Ascending);
        assert!(s.selected_name.is_none());
    }

    /// Given: cwd is saved
    /// When: load_from() is called
    /// Then: cwd is restored
    #[test]
    fn save_then_load_restores_cwd() {
        let path = temp_session_path("cwd");
        let cwd = std::env::temp_dir();
        save_defaults(&path, &cwd);
        let s = load_from(&path);
        assert_eq!(s.cwd, Some(cwd));
    }

    /// Given: marks are saved
    /// When: load_from() is called
    /// Then: marks are restored
    #[test]
    fn save_then_load_restores_marks() {
        let path = temp_session_path("marks");
        let cwd = std::env::temp_dir();
        let mut marks = HashMap::new();
        marks.insert('a', cwd.clone());
        save_to(
            &path,
            &cwd,
            &marks,
            false,
            SortMode::default(),
            SortOrder::default(),
            None,
        )
        .unwrap();
        let s = load_from(&path);
        assert_eq!(s.marks.get(&'a'), Some(&cwd));
    }

    /// Given: session file contains a cwd that no longer exists
    /// When: load_from() is called
    /// Then: cwd is None
    #[test]
    fn load_skips_missing_cwd_directory() {
        let path = temp_session_path("gone_cwd");
        let gone = PathBuf::from("/tmp/__trek_gone_cwd_test__");
        save_defaults(&path, &gone);
        let s = load_from(&path);
        assert!(s.cwd.is_none());
    }

    /// Given: session file contains a mark pointing to a deleted path
    /// When: load_from() is called
    /// Then: that mark is omitted
    #[test]
    fn load_skips_missing_mark_paths() {
        let path = temp_session_path("gone_mark");
        let cwd = std::env::temp_dir();
        let mut marks = HashMap::new();
        marks.insert('z', PathBuf::from("/tmp/__trek_no_such_mark__"));
        save_to(
            &path,
            &cwd,
            &marks,
            false,
            SortMode::default(),
            SortOrder::default(),
            None,
        )
        .unwrap();
        let s = load_from(&path);
        assert!(s.marks.get(&'z').is_none());
    }

    /// session_path() always produces a path ending with "trek/session" regardless
    /// of environment — no env mutation needed to verify the suffix.
    #[test]
    fn session_path_uses_xdg_data_home() {
        let p = session_path();
        assert!(p.ends_with("trek/session"), "got: {}", p.display());
    }

    /// Given: show_hidden=true is saved
    /// When: load_from() is called
    /// Then: show_hidden is true
    #[test]
    fn save_then_load_restores_show_hidden() {
        let path = temp_session_path("hidden");
        let cwd = std::env::temp_dir();
        save_to(
            &path,
            &cwd,
            &HashMap::new(),
            true,
            SortMode::default(),
            SortOrder::default(),
            None,
        )
        .unwrap();
        let s = load_from(&path);
        assert!(s.show_hidden);
    }

    /// Given: sort_mode=Modified is saved
    /// When: load_from() is called
    /// Then: sort_mode is Modified
    #[test]
    fn save_then_load_restores_sort_mode() {
        let path = temp_session_path("sort_mode");
        let cwd = std::env::temp_dir();
        save_to(
            &path,
            &cwd,
            &HashMap::new(),
            false,
            SortMode::Modified,
            SortOrder::default(),
            None,
        )
        .unwrap();
        let s = load_from(&path);
        assert_eq!(s.sort_mode, SortMode::Modified);
    }

    /// Given: sort_order=Descending is saved
    /// When: load_from() is called
    /// Then: sort_order is Descending
    #[test]
    fn save_then_load_restores_sort_order() {
        let path = temp_session_path("sort_order");
        let cwd = std::env::temp_dir();
        save_to(
            &path,
            &cwd,
            &HashMap::new(),
            false,
            SortMode::default(),
            SortOrder::Descending,
            None,
        )
        .unwrap();
        let s = load_from(&path);
        assert_eq!(s.sort_order, SortOrder::Descending);
    }

    /// Given: a selected entry name is saved
    /// When: load_from() is called
    /// Then: selected_name is restored
    #[test]
    fn save_then_load_restores_selected_name() {
        let path = temp_session_path("selected");
        let cwd = std::env::temp_dir();
        save_to(
            &path,
            &cwd,
            &HashMap::new(),
            false,
            SortMode::default(),
            SortOrder::default(),
            Some("Cargo.toml"),
        )
        .unwrap();
        let s = load_from(&path);
        assert_eq!(s.selected_name.as_deref(), Some("Cargo.toml"));
    }

    /// Given: a session file written with unknown keys (future version)
    /// When: load_from() is called
    /// Then: known fields are parsed, unknown keys are silently ignored
    #[test]
    fn load_ignores_unknown_keys_for_forward_compat() {
        let path = temp_session_path("compat");
        let cwd = std::env::temp_dir();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "cwd={}\nshow_hidden=false\nunknown_future_key=xyz\n",
                cwd.display()
            ),
        )
        .unwrap();
        let s = load_from(&path);
        assert_eq!(s.cwd, Some(cwd));
        assert!(!s.show_hidden);
    }
}
