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
    load_from(&bookmarks_path())
}

/// Load bookmarks from an explicit file path.
///
/// Identical to `load()` but accepts an explicit path rather than reading from
/// the environment, making it usable in tests without mutating env vars.
pub(crate) fn load_from(path: &Path) -> Vec<PathBuf> {
    let Ok(file) = std::fs::File::open(path) else {
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
    add_to(&bookmarks_path(), dir)
}

/// Add `dir` to the bookmarks list at an explicit path.
pub(crate) fn add_to(path: &Path, dir: &Path) -> std::io::Result<()> {
    let mut bms = load_from(path);
    if bms.iter().any(|b| b == dir) {
        return Ok(());
    }
    bms.push(dir.to_path_buf());
    save_to(path, &bms)
}

/// Remove the bookmark at `index` (into the list returned by `load()`).
/// Out-of-range indices are silently ignored.
pub fn remove(index: usize) -> std::io::Result<()> {
    remove_from(&bookmarks_path(), index)
}

/// Remove the bookmark at `index` from an explicit path.
pub(crate) fn remove_from(path: &Path, index: usize) -> std::io::Result<()> {
    let mut bms = load_from(path);
    if index < bms.len() {
        bms.remove(index);
        save_to(path, &bms)?;
    }
    Ok(())
}

/// Write `bms` to an explicit file path, creating parent directories as needed.
pub(crate) fn save_to(path: &Path, bms: &[PathBuf]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    for b in bms {
        writeln!(f, "{}", b.display())?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Return a unique per-test temporary bookmarks file path. No env mutation needed.
    fn temp_bookmarks_path(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("trek_bm_{}_{}_{}", std::process::id(), n, tag))
            .join("trek")
            .join("bookmarks")
    }

    /// Given: no bookmarks file exists
    /// When: load_from() is called
    /// Then: an empty Vec is returned without panicking
    #[test]
    fn load_returns_empty_when_no_file() {
        let path = temp_bookmarks_path("empty");
        let bms = load_from(&path);
        assert!(bms.is_empty());
    }

    /// Given: a directory is added as a bookmark
    /// When: load_from() is called
    /// Then: the bookmark is present in the returned list
    #[test]
    fn add_then_load_returns_bookmark() {
        let path = temp_bookmarks_path("add");
        let dir = std::env::temp_dir();
        add_to(&path, &dir).unwrap();
        let bms = load_from(&path);
        assert!(bms.contains(&dir), "bookmark should be present after add");
    }

    /// Given: the same directory is added twice
    /// When: load_from() is called
    /// Then: only one entry exists (silent deduplication)
    #[test]
    fn add_deduplicates_silently() {
        let path = temp_bookmarks_path("dedup");
        let dir = std::env::temp_dir();
        add_to(&path, &dir).unwrap();
        add_to(&path, &dir).unwrap();
        let bms = load_from(&path);
        let count = bms.iter().filter(|b| *b == &dir).count();
        assert_eq!(count, 1, "duplicate should be silently ignored");
    }

    /// Given: two bookmarks are saved, then remove_from(0) is called
    /// When: load_from() is called
    /// Then: only the second bookmark remains
    #[test]
    fn remove_at_index_zero_removes_first() {
        let path = temp_bookmarks_path("remove");
        let a = PathBuf::from("/tmp/trek_bm_a");
        let b = PathBuf::from("/tmp/trek_bm_b");
        save_to(&path, &[a.clone(), b.clone()]).unwrap();
        remove_from(&path, 0).unwrap();
        let bms = load_from(&path);
        assert_eq!(bms, vec![b]);
    }

    /// Given: an out-of-range index is passed to remove_from()
    /// When: remove_from() is called
    /// Then: it returns Ok(()) without panicking or modifying the list
    #[test]
    fn remove_out_of_range_is_noop() {
        let path = temp_bookmarks_path("oob");
        let dir = std::env::temp_dir();
        add_to(&path, &dir).unwrap();
        assert!(remove_from(&path, 99).is_ok());
        let bms = load_from(&path);
        assert_eq!(bms.len(), 1, "list should be unchanged");
    }

    /// bookmarks_path() always produces a path ending with "trek/bookmarks"
    /// regardless of environment — no env mutation needed to verify the suffix.
    #[test]
    fn bookmarks_path_uses_xdg_data_home() {
        let path = bookmarks_path();
        assert!(path.ends_with("trek/bookmarks"), "got: {}", path.display());
    }
}
