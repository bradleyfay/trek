//! Session persistence — save and restore cwd and marks between Trek invocations.
//!
//! Stored at `$XDG_DATA_HOME/trek/session` (fallback: `~/.local/share/trek/session`).
//! Simple `key=value` format; no external dependencies.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub struct Session {
    /// Restored working directory (`None` if file missing or dir deleted).
    pub cwd: Option<PathBuf>,
    /// Restored mark slots; only entries where the path still exists are included.
    pub marks: HashMap<char, PathBuf>,
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

/// Load the session file. Returns `Session { cwd: None, marks: {} }` if the
/// file is absent or unreadable — never panics.
pub fn load() -> Session {
    let Ok(file) = std::fs::File::open(session_path()) else {
        return Session {
            cwd: None,
            marks: HashMap::new(),
        };
    };
    let mut cwd = None;
    let mut marks = HashMap::new();
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let line = line.trim().to_owned();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let path = PathBuf::from(val);
        if key == "cwd" {
            if path.is_dir() {
                cwd = Some(path);
            }
        } else if let Some(letter) = key.strip_prefix("mark.") {
            if let Some(c) = letter.chars().next().filter(|ch| ch.is_alphabetic()) {
                if path.exists() {
                    marks.insert(c, path);
                }
            }
        }
    }
    Session { cwd, marks }
}

/// Write `cwd` and `marks` to the session file. Errors are silently ignored
/// at call sites — a failed save should never crash Trek.
pub fn save(cwd: &Path, marks: &HashMap<char, PathBuf>) -> std::io::Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(&path)?;
    writeln!(f, "cwd={}", cwd.display())?;
    let mut sorted: Vec<_> = marks.iter().collect();
    sorted.sort_by_key(|(c, _)| *c);
    for (c, p) in sorted {
        writeln!(f, "mark.{}={}", c, p.display())?;
    }
    Ok(())
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

    /// Given: no session file exists
    /// When: load() is called
    /// Then: returns empty session without panicking
    #[test]
    fn load_returns_empty_session_when_no_file() {
        with_temp_session(|| {
            let s = load();
            assert!(s.cwd.is_none());
            assert!(s.marks.is_empty());
        });
    }

    /// Given: cwd and a mark are saved
    /// When: load() is called
    /// Then: cwd and mark are restored
    #[test]
    fn save_then_load_restores_cwd() {
        with_temp_session(|| {
            let tmp = std::env::temp_dir();
            let marks = HashMap::new();
            save(&tmp, &marks).unwrap();
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
            save(&tmp, &marks).unwrap();
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
            let marks = HashMap::new();
            save(&gone, &marks).unwrap();
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
            save(&tmp, &marks).unwrap();
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
}
