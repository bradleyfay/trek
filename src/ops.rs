use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ── Clipboard model ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
pub struct Clipboard {
    pub op: ClipboardOp,
    pub paths: Vec<PathBuf>,
}

// ── Filesystem operations ──────────────────────────────────────────────────────

/// Copy a single file or directory (recursively) from `src` to `dst`.
///
/// `dst` is the full destination path, not a parent directory.
pub fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create parent dirs for {:?}", dst))?;
        }
        std::fs::copy(src, dst).with_context(|| format!("copy {:?} -> {:?}", src, dst))?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("create_dir_all {:?}", dst))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("read_dir {:?}", src))? {
        let entry = entry?;
        let src_child = entry.path();
        let dst_child = dst.join(entry.file_name());
        if src_child.is_dir() {
            copy_dir_recursive(&src_child, &dst_child)?;
        } else {
            std::fs::copy(&src_child, &dst_child)
                .with_context(|| format!("copy {:?} -> {:?}", src_child, dst_child))?;
        }
    }
    Ok(())
}

/// Move `src` to `dst`. Tries atomic rename first; falls back to copy+delete
/// for cross-device moves.
///
/// `dst` is the full destination path.
pub fn move_path(src: &Path, dst: &Path) -> Result<()> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Cross-device or other rename failure: copy then delete.
            copy_path(src, dst)?;
            delete_path(src)
        }
    }
}

/// Delete a file or directory (recursively if directory).
pub fn delete_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).with_context(|| format!("remove_dir_all {:?}", path))
    } else {
        std::fs::remove_file(path).with_context(|| format!("remove_file {:?}", path))
    }
}

/// Create a directory named `name` inside `parent`.
///
/// Returns the created path. Fails if `name` already exists.
pub fn make_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    let path = parent.join(name);
    std::fs::create_dir(&path).with_context(|| format!("mkdir {:?}", path))?;
    Ok(path)
}

/// Create a new empty file named `name` inside `parent`.
///
/// Returns the created path. Fails if a file with that name already exists,
/// preventing silent overwrites.
pub fn touch_file(parent: &Path, name: &str) -> Result<PathBuf> {
    let path = parent.join(name);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("touch {:?}", path))?;
    Ok(path)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Return a per-test-run temporary directory (unique on every invocation).
    fn tmp_dir(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let tid = format!("{:?}", std::thread::current().id());
        let dir = std::env::temp_dir().join(format!(
            "trek_ops_{}_{}_{}",
            label,
            ts,
            tid.replace(['(', ')', ' '], "")
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// Given: a file exists at src
    /// When: copy_path is called to a new path in the same directory
    /// Then: both src and dst exist with the same content
    #[test]
    fn copy_file_same_dir() {
        let tmp = tmp_dir("copy_same");
        let src = tmp.join("a.txt");
        let dst = tmp.join("b.txt");
        write_file(&src, "hello");
        copy_path(&src, &dst).unwrap();
        assert!(src.exists(), "src should still exist");
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
    }

    /// Given: a file exists at src
    /// When: copy_path copies it to a subdirectory
    /// Then: dst exists; src still exists
    #[test]
    fn copy_file_different_dir() {
        let tmp = tmp_dir("copy_diff");
        let src = tmp.join("orig.txt");
        let dst = tmp.join("sub").join("orig.txt");
        write_file(&src, "world");
        copy_path(&src, &dst).unwrap();
        assert!(src.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "world");
    }

    /// Given: a directory with nested files and subdirectories
    /// When: copy_path is called (recursive directory copy)
    /// Then: dst contains all nested files with the same content; src is unchanged
    #[test]
    fn copy_dir_recursive() {
        let tmp = tmp_dir("copy_dir");
        let src_dir = tmp.join("srcdir");
        let nested = src_dir.join("nested");
        fs::create_dir_all(&nested).unwrap();
        write_file(&src_dir.join("top.txt"), "top");
        write_file(&nested.join("deep.txt"), "deep");
        let dst_dir = tmp.join("dstdir");
        copy_path(&src_dir, &dst_dir).unwrap();
        assert_eq!(fs::read_to_string(dst_dir.join("top.txt")).unwrap(), "top");
        assert_eq!(
            fs::read_to_string(dst_dir.join("nested").join("deep.txt")).unwrap(),
            "deep"
        );
        // Original is untouched.
        assert!(src_dir.join("top.txt").exists());
    }

    /// Given: a file exists at src
    /// When: move_path is called to a new location on the same filesystem
    /// Then: src no longer exists; dst has the original content
    #[test]
    fn move_file_same_fs() {
        let tmp = tmp_dir("move_same");
        let src = tmp.join("mv_src.txt");
        let dst = tmp.join("mv_dst.txt");
        write_file(&src, "move me");
        move_path(&src, &dst).unwrap();
        assert!(!src.exists(), "src should be gone after move");
        assert_eq!(fs::read_to_string(&dst).unwrap(), "move me");
    }

    /// Given: a file exists at path
    /// When: delete_path is called
    /// Then: the file no longer exists
    #[test]
    fn delete_file_removes_it() {
        let tmp = tmp_dir("del_file");
        let path = tmp.join("todelete.txt");
        write_file(&path, "delete me");
        delete_path(&path).unwrap();
        assert!(!path.exists());
    }

    /// Given: a directory with contents exists at path
    /// When: delete_path is called
    /// Then: the directory and all its contents no longer exist
    #[test]
    fn delete_dir_removes_recursively() {
        let tmp = tmp_dir("del_dir");
        let dir = tmp.join("rmdir");
        fs::create_dir_all(dir.join("sub")).unwrap();
        write_file(&dir.join("file.txt"), "x");
        delete_path(&dir).unwrap();
        assert!(!dir.exists());
    }

    /// Given: a valid parent directory
    /// When: make_dir is called with a name that does not exist
    /// Then: the directory is created and the full path returned
    #[test]
    fn make_dir_creates_directory() {
        let tmp = tmp_dir("mkdir");
        let result = make_dir(&tmp, "newdir").unwrap();
        assert!(result.is_dir());
        assert_eq!(result, tmp.join("newdir"));
    }

    /// Given: a directory already exists with the given name
    /// When: make_dir is called with the same name
    /// Then: an error is returned (no silent overwrite)
    #[test]
    fn make_dir_errors_if_already_exists() {
        let tmp = tmp_dir("mkdir_exists");
        make_dir(&tmp, "existing").unwrap(); // first call succeeds
        let result = make_dir(&tmp, "existing"); // second should fail
        assert!(result.is_err());
    }
}
