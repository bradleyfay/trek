//! Persistent directory bookmarks.
//!
//! Bookmarks are stored at `$XDG_DATA_HOME/trek/bookmarks` (falling back to
//! `~/.local/share/trek/bookmarks`) — one absolute path per line, in
//! insertion order.  No external crate dependencies.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Return the path of the bookmarks file.
pub fn bookmarks_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/share")
        });
    base.join("trek").join("bookmarks")
}

/// Load bookmarks from disk.  Returns an empty `Vec` if the file does not
/// exist or cannot be read.
pub fn load() -> Vec<PathBuf> {
    let Ok(file) = std::fs::File::open(bookmarks_path()) else {
        return Vec::new();
    };
    std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Add `dir` to the bookmarks list.  Silently deduplicates — if `dir` is
/// already bookmarked, this is a no-op.
pub fn add(dir: &Path) -> std::io::Result<()> {
    let mut bms = load();
    if bms.iter().any(|b| b == dir) {
        return Ok(());
    }
    bms.push(dir.to_path_buf());
    save(&bms)
}

/// Remove the bookmark at `index` (into the list returned by `load()`).
/// Out-of-range indices are silently ignored.
pub fn remove(index: usize) -> std::io::Result<()> {
    let mut bms = load();
    if index < bms.len() {
        bms.remove(index);
        save(&bms)?;
    }
    Ok(())
}

/// Write `bms` to disk, creating parent directories as needed.
fn save(bms: &[PathBuf]) -> std::io::Result<()> {
    let path = bookmarks_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(&path)?;
    for b in bms {
        writeln!(f, "{}", b.display())?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serializes all bookmark tests that mutate `XDG_DATA_HOME`.
    /// Env var mutation is process-global, so tests touching it must not run
    /// concurrently.
    static BM_LOCK: Mutex<()> = Mutex::new(());

    /// Run `f` inside a temp directory used as `XDG_DATA_HOME`, restoring the
    /// previous value afterwards.
    fn with_temp_bookmarks<F: FnOnce(&Path)>(f: F) {
        let _guard = BM_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Use a pid+counter suffix to avoid clashing between parallel test
        // binaries (different processes can share /tmp).
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let tmp = std::env::temp_dir().join(format!("trek_bm_test_{}_{}", std::process::id(), n));
        let _ = fs::create_dir_all(&tmp);

        let prev = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", &tmp);
        f(&tmp);
        match prev {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: no bookmarks file exists
    /// When: load() is called
    /// Then: an empty Vec is returned without panicking
    #[test]
    fn load_returns_empty_when_no_file() {
        with_temp_bookmarks(|_| {
            let bms = load();
            assert!(bms.is_empty());
        });
    }

    /// Given: a directory is added as a bookmark
    /// When: load() is called
    /// Then: the bookmark is present in the returned list
    #[test]
    fn add_then_load_returns_bookmark() {
        with_temp_bookmarks(|_| {
            let dir = std::env::temp_dir();
            add(&dir).unwrap();
            let bms = load();
            assert!(bms.contains(&dir), "bookmark should be present after add");
        });
    }

    /// Given: the same directory is added twice
    /// When: load() is called
    /// Then: only one entry exists (silent deduplication)
    #[test]
    fn add_deduplicates_silently() {
        with_temp_bookmarks(|_| {
            let dir = std::env::temp_dir();
            add(&dir).unwrap();
            add(&dir).unwrap();
            let bms = load();
            let count = bms.iter().filter(|b| *b == &dir).count();
            assert_eq!(count, 1, "duplicate should be silently ignored");
        });
    }

    /// Given: two bookmarks are saved, then remove(0) is called
    /// When: load() is called
    /// Then: only the second bookmark remains
    #[test]
    fn remove_at_index_zero_removes_first() {
        with_temp_bookmarks(|_| {
            let a = PathBuf::from("/tmp/trek_bm_a");
            let b = PathBuf::from("/tmp/trek_bm_b");
            save(&[a.clone(), b.clone()]).unwrap();
            remove(0).unwrap();
            let bms = load();
            assert_eq!(bms, vec![b]);
        });
    }

    /// Given: an out-of-range index is passed to remove()
    /// When: remove() is called
    /// Then: it returns Ok(()) without panicking or modifying the list
    #[test]
    fn remove_out_of_range_is_noop() {
        with_temp_bookmarks(|_| {
            let dir = std::env::temp_dir();
            add(&dir).unwrap();
            assert!(remove(99).is_ok());
            let bms = load();
            assert_eq!(bms.len(), 1, "list should be unchanged");
        });
    }

    /// Given: bookmarks_path() is called with XDG_DATA_HOME set
    /// When: the path is inspected
    /// Then: it starts with the XDG_DATA_HOME value and ends with trek/bookmarks
    #[test]
    fn bookmarks_path_uses_xdg_data_home() {
        with_temp_bookmarks(|tmp| {
            let path = bookmarks_path();
            assert!(
                path.starts_with(tmp),
                "expected path under {}, got {}",
                tmp.display(),
                path.display()
            );
            assert!(path.ends_with("trek/bookmarks"));
        });
    }
}
