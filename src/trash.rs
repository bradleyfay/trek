//! Soft-delete (trash) support for trek.
//!
//! Files are moved to the platform trash directory rather than permanently
//! deleted.  A single undo group is held in `App::last_trashed` so that
//! `u` can restore the most recent group.
//!
//! Platform behaviour:
//! - **macOS**: `~/.Trash/`
//! - **Linux**: `$XDG_DATA_HOME/Trash/files/` with `.trashinfo` sidecars in
//!   `$XDG_DATA_HOME/Trash/info/` for Nautilus/Thunar compatibility.
//!   (Full FreeDesktop per-device `$TOPDIR/.Trash-<uid>` support is a
//!   TODO for a future release.)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ── Public types ──────────────────────────────────────────────────────────────

/// A record of one item moved to the trash, used for undo.
#[derive(Clone, Debug)]
pub struct TrashedEntry {
    /// Where the file lived before it was trashed.
    pub original: PathBuf,
    /// Where the file is now (inside the trash directory).
    pub trash_dest: PathBuf,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Move `path` to the platform trash directory and return the entry.
///
/// Collision handling: appends ` (2)`, ` (3)`, … to the stem until a free
/// slot is found (up to 100 attempts).
pub fn trash_path(path: &Path) -> Result<TrashedEntry> {
    let trash_dir = platform_trash_dir()?;
    std::fs::create_dir_all(&trash_dir)
        .with_context(|| format!("create trash dir {:?}", trash_dir))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("path has no filename: {:?}", path))?;

    let dest = unique_trash_dest(&trash_dir, file_name)?;

    #[cfg(target_os = "linux")]
    write_trashinfo(&dest, path)?;

    // Attempt atomic rename first; fall back to copy+delete for cross-device.
    if std::fs::rename(path, &dest).is_err() {
        crate::ops::copy_path(path, &dest)
            .with_context(|| format!("cross-device trash copy {:?}", path))?;
        crate::ops::delete_path(path)
            .with_context(|| format!("cross-device trash remove {:?}", path))?;
    }

    Ok(TrashedEntry {
        original: path.to_owned(),
        trash_dest: dest,
    })
}

/// Restore a previously trashed item back to its original path.
///
/// Returns an error (and leaves the file in the trash) if the trash slot no
/// longer exists, or if the original parent directory cannot be created.
pub fn restore_path(entry: &TrashedEntry) -> Result<()> {
    if !entry.trash_dest.exists() {
        anyhow::bail!("file no longer in trash: {}", entry.trash_dest.display());
    }
    if let Some(parent) = entry.original.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("restore parent dirs {:?}", parent))?;
    }
    std::fs::rename(&entry.trash_dest, &entry.original)
        .with_context(|| format!("restore {:?} -> {:?}", entry.trash_dest, entry.original))?;

    #[cfg(target_os = "linux")]
    {
        let _ = std::fs::remove_file(trashinfo_path(&entry.trash_dest));
    }

    Ok(())
}

/// Return the platform-appropriate trash directory.
///
/// - **macOS**: `~/.Trash`
/// - **Linux/other**: `$XDG_DATA_HOME/Trash/files` (default: `~/.local/share/Trash/files`)
pub fn platform_trash_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = home_dir()?;
        Ok(home.join(".Trash"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().unwrap_or_default().join(".local/share"));
        Ok(base.join("Trash/files"))
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("$HOME is not set"))
}

/// Find a free filename in `trash_dir`, appending ` (N)` on collision.
pub fn unique_trash_dest(trash_dir: &Path, file_name: &std::ffi::OsStr) -> Result<PathBuf> {
    let name = file_name.to_string_lossy();
    // Split at the *last* dot, but treat a leading dot as part of the stem
    // (e.g. `.gitignore` → stem=".gitignore", ext="").
    let (stem, ext): (&str, &str) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name.as_ref(), ""),
    };

    for n in 1..=100u32 {
        let candidate = if n == 1 {
            trash_dir.join(&*name)
        } else {
            trash_dir.join(format!("{} ({}){}", stem, n, ext))
        };
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "could not find a free trash slot for {:?} after 100 attempts",
        file_name
    )
}

// ── Linux-only: FreeDesktop Trash specification ───────────────────────────────

/// Path of the `.trashinfo` sidecar for `trash_dest`.
#[cfg(target_os = "linux")]
fn trashinfo_path(trash_dest: &Path) -> PathBuf {
    trash_dest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("info"))
        .unwrap_or_default()
        .join(format!(
            "{}.trashinfo",
            trash_dest.file_name().unwrap_or_default().to_string_lossy()
        ))
}

/// Write a `.trashinfo` sidecar conforming to the FreeDesktop Trash spec.
#[cfg(target_os = "linux")]
fn write_trashinfo(trash_dest: &Path, original: &Path) -> Result<()> {
    let info_path = trashinfo_path(trash_dest);
    if let Some(parent) = info_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let content = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        original.display(),
        format_iso8601_utc(secs)
    );
    std::fs::write(&info_path, content).with_context(|| format!("write trashinfo {:?}", info_path))
}

/// Format a UNIX timestamp as `YYYY-MM-DDTHH:MM:SS` (UTC) using pure Rust
/// arithmetic — no subprocess, no external crates.
#[cfg(target_os = "linux")]
fn format_iso8601_utc(secs: u64) -> String {
    let ss = secs % 60;
    let mm = (secs / 60) % 60;
    let hh = (secs / 3600) % 24;
    let mut days = secs / 86_400;

    let mut year = 1970u32;
    loop {
        let dy = if is_leap_year(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }

    let month_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hh, mm, ss
    )
}

#[cfg(target_os = "linux")]
fn is_leap_year(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Given: an empty trash directory and a filename with no collision
    /// When: unique_trash_dest is called
    /// Then: returns `trash_dir/filename` unchanged
    #[test]
    fn unique_dest_no_collision() {
        let tmp = std::env::temp_dir().join(format!("trek_trash_test_nc_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let name = std::ffi::OsStr::new("test_file.txt");
        let dest = unique_trash_dest(&tmp, name).unwrap();
        let _ = fs::remove_dir_all(&tmp);
        assert_eq!(dest, tmp.join("test_file.txt"));
    }

    /// Given: a trash directory that already contains the target filename
    /// When: unique_trash_dest is called
    /// Then: returns `stem (2).ext`
    #[test]
    fn unique_dest_one_collision() {
        let tmp = std::env::temp_dir().join(format!("trek_trash_test_1c_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        fs::write(tmp.join("doc.txt"), b"existing").unwrap();
        let name = std::ffi::OsStr::new("doc.txt");
        let dest = unique_trash_dest(&tmp, name).unwrap();
        let _ = fs::remove_dir_all(&tmp);
        assert_eq!(dest, tmp.join("doc (2).txt"));
    }

    /// Given: a dotfile name (leading dot, no extension)
    /// When: unique_trash_dest is called with no collision
    /// Then: returns the dotfile name unchanged (leading dot is part of stem)
    #[test]
    fn unique_dest_dotfile() {
        let tmp = std::env::temp_dir().join(format!("trek_trash_test_df_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let name = std::ffi::OsStr::new(".gitignore");
        let dest = unique_trash_dest(&tmp, name).unwrap();
        let _ = fs::remove_dir_all(&tmp);
        assert_eq!(dest, tmp.join(".gitignore"));
    }

    /// Given: a regular file in a temp directory
    /// When: trash_path is called, then restore_path is called
    /// Then: file is at original path and absent from trash
    #[test]
    fn trash_then_restore_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("trek_trash_test_rt_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let src = tmp.join("hello.txt");
        fs::write(&src, b"hello").unwrap();

        let entry = trash_path(&src).unwrap();
        assert!(!src.exists(), "original should be gone after trash");
        assert!(entry.trash_dest.exists(), "file should exist at trash dest");

        restore_path(&entry).unwrap();
        assert!(src.exists(), "file should be restored to original path");
        assert!(
            !entry.trash_dest.exists(),
            "trash slot should be empty after restore"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a TrashedEntry whose trash_dest no longer exists
    /// When: restore_path is called
    /// Then: an error is returned
    #[test]
    fn restore_missing_file_returns_error() {
        let entry = TrashedEntry {
            original: PathBuf::from("/tmp/trek_restore_orig_nonexistent"),
            trash_dest: PathBuf::from("/tmp/trek_restore_dest_nonexistent_xyz123"),
        };
        let result = restore_path(&entry);
        assert!(result.is_err(), "restoring a missing file should error");
    }

    /// Given: platform_trash_dir() is called
    /// When: the result is inspected
    /// Then: it returns Ok with a non-empty path
    #[test]
    fn platform_trash_dir_returns_path() {
        let dir = platform_trash_dir().unwrap();
        assert!(!dir.as_os_str().is_empty());
    }

    /// Given: format_iso8601_utc is called with a known UNIX timestamp
    /// When: the result is compared to the expected ISO 8601 string
    /// Then: the format is correct (YYYY-MM-DDTHH:MM:SS)
    #[cfg(target_os = "linux")]
    #[test]
    fn format_iso8601_known_timestamp() {
        // 2024-01-15T12:30:45 UTC = 1705318245
        assert_eq!(format_iso8601_utc(1_705_318_245), "2024-01-15T12:30:45");
    }
}
