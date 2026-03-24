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
    let Ok(file) = std::fs::File::open(session_path()) else {
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
    let path = session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(&path)?;
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
    use std::sync::Mutex;

    static SESSION_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_session<F: FnOnce()>(f: F) {
        let _guard = SESSION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = std::env::temp_dir().join(format!("trek_sess_mod_{}_{}", std::process::id(), n));
        let _ = std::fs::create_dir_all(&tmp);
        let prev = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", &tmp);
        f();
        match prev {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Helper: save with all fields set to their defaults.
    fn save_defaults(dir: &std::path::Path) {
        save(
            dir,
            &HashMap::new(),
            false,
            SortMode::default(),
            SortOrder::default(),
            None,
        )
        .unwrap();
    }

    /// Given: no session file exists
    /// When: load() is called
    /// Then: returns empty session without panicking
    #[test]
    fn load_returns_empty_session_when_no_file() {
        with_temp_session(|| {
            let s = load();
            assert!(s.cwd.is_none());
            assert!(s.marks.is_empty());
            assert!(!s.show_hidden);
            assert_eq!(s.sort_mode, SortMode::Name);
            assert_eq!(s.sort_order, SortOrder::Ascending);
            assert!(s.selected_name.is_none());
        });
    }

    /// Given: cwd and a mark are saved
    /// When: load() is called
    /// Then: cwd and mark are restored
    #[test]
    fn save_then_load_restores_cwd() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            save_defaults(&tmp);
            let s = load();
            assert_eq!(s.cwd, Some(tmp));
        });
    }

    /// Given: marks are saved
    /// When: load() is called
    /// Then: marks are restored
    #[test]
    fn save_then_load_restores_marks() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            let mut marks = HashMap::new();
            marks.insert('a', tmp.clone());
            save(
                &tmp,
                &marks,
                false,
                SortMode::default(),
                SortOrder::default(),
                None,
            )
            .unwrap();
            let s = load();
            assert_eq!(s.marks.get(&'a'), Some(&tmp));
        });
    }

    /// Given: session file contains a cwd that no longer exists
    /// When: load() is called
    /// Then: cwd is None
    #[test]
    fn load_skips_missing_cwd_directory() {
        with_temp_session(|| {
            let gone = PathBuf::from("/tmp/__trek_gone_cwd_test__");
            save_defaults(&gone);
            let s = load();
            assert!(s.cwd.is_none());
        });
    }

    /// Given: session file contains a mark pointing to a deleted path
    /// When: load() is called
    /// Then: that mark is omitted
    #[test]
    fn load_skips_missing_mark_paths() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            let mut marks = HashMap::new();
            marks.insert('z', PathBuf::from("/tmp/__trek_no_such_mark__"));
            save(
                &tmp,
                &marks,
                false,
                SortMode::default(),
                SortOrder::default(),
                None,
            )
            .unwrap();
            let s = load();
            assert!(s.marks.get(&'z').is_none());
        });
    }

    /// Given: session_path() is called with XDG_DATA_HOME set
    /// When: the path is inspected
    /// Then: it ends with trek/session
    #[test]
    fn session_path_uses_xdg_data_home() {
        with_temp_session(|| {
            let p = session_path();
            assert!(p.ends_with("trek/session"), "got: {}", p.display());
        });
    }

    /// Given: show_hidden=true is saved
    /// When: load() is called
    /// Then: show_hidden is true
    #[test]
    fn save_then_load_restores_show_hidden() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            save(
                &tmp,
                &HashMap::new(),
                true,
                SortMode::default(),
                SortOrder::default(),
                None,
            )
            .unwrap();
            let s = load();
            assert!(s.show_hidden);
        });
    }

    /// Given: sort_mode=Modified is saved
    /// When: load() is called
    /// Then: sort_mode is Modified
    #[test]
    fn save_then_load_restores_sort_mode() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            save(
                &tmp,
                &HashMap::new(),
                false,
                SortMode::Modified,
                SortOrder::default(),
                None,
            )
            .unwrap();
            let s = load();
            assert_eq!(s.sort_mode, SortMode::Modified);
        });
    }

    /// Given: sort_order=Descending is saved
    /// When: load() is called
    /// Then: sort_order is Descending
    #[test]
    fn save_then_load_restores_sort_order() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            save(
                &tmp,
                &HashMap::new(),
                false,
                SortMode::default(),
                SortOrder::Descending,
                None,
            )
            .unwrap();
            let s = load();
            assert_eq!(s.sort_order, SortOrder::Descending);
        });
    }

    /// Given: a selected entry name is saved
    /// When: load() is called
    /// Then: selected_name is restored
    #[test]
    fn save_then_load_restores_selected_name() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            save(
                &tmp,
                &HashMap::new(),
                false,
                SortMode::default(),
                SortOrder::default(),
                Some("Cargo.toml"),
            )
            .unwrap();
            let s = load();
            assert_eq!(s.selected_name.as_deref(), Some("Cargo.toml"));
        });
    }

    /// Given: a session file written with unknown keys (future version)
    /// When: load() is called
    /// Then: known fields are parsed, unknown keys are silently ignored
    #[test]
    fn load_ignores_unknown_keys_for_forward_compat() {
        with_temp_session(|| {
            let path = session_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let tmp = std::env::temp_dir();
            std::fs::write(
                &path,
                format!(
                    "cwd={}\nshow_hidden=false\nunknown_future_key=xyz\n",
                    tmp.display()
                ),
            )
            .unwrap();
            let s = load();
            assert_eq!(s.cwd, Some(tmp));
            assert!(!s.show_hidden);
        });
    }
}
