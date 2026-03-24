/// Session snapshot — captures filesystem state at a point in time and computes
/// a diff against the current state to show what changed during a session.
///
/// This module is intentionally self-contained: it owns its own data types and
/// has no dependency on the rest of App beyond what is passed explicitly.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Maximum number of changed files reported in a single diff.
/// Entries beyond this limit are represented by a summary count only.
pub const MAX_DIFF_ENTRIES: usize = 200;

/// Recorded metadata for one file at snapshot time.
#[derive(Clone)]
pub struct SnapshotEntry {
    pub mtime: SystemTime,
    pub size: u64,
}

/// Classification of a file's change since the snapshot.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ChangeKind {
    /// File did not exist at snapshot time.
    New,
    /// File existed and its mtime or size changed.
    Modified,
    /// File existed at snapshot time but is gone now.
    Deleted,
}

/// One changed file in the session diff.
#[derive(Clone, Debug)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub kind: ChangeKind,
    /// Current size in bytes (0 for deleted files).
    pub size: u64,
    /// Snapshot size in bytes (0 for new files).
    pub old_size: u64,
}

/// A point-in-time snapshot of a directory tree.
pub struct SessionSnapshot {
    /// When this snapshot was taken.
    pub taken_at: SystemTime,
    /// The root directory that was walked.
    pub root: PathBuf,
    /// Map of relative path → recorded metadata at snapshot time.
    pub entries: HashMap<PathBuf, SnapshotEntry>,
}

impl SessionSnapshot {
    /// Capture a snapshot by walking `root` recursively.
    ///
    /// Hidden files and directories are always included so that toggling the
    /// hidden-files display mid-session does not create artificial gaps.
    /// Symlinks are not followed to avoid cycles.
    pub fn capture(root: &Path) -> Self {
        let entries = walk_tree(root);
        Self {
            taken_at: SystemTime::now(),
            root: root.to_path_buf(),
            entries,
        }
    }

    /// Compute the diff between this snapshot and the current filesystem state.
    ///
    /// Returns up to `MAX_DIFF_ENTRIES` changed files, sorted:
    /// New first (alphabetical), then Modified (alphabetical), then Deleted (alphabetical).
    ///
    /// The caller can detect truncation by checking whether the returned length
    /// equals `MAX_DIFF_ENTRIES` and `total_changed > MAX_DIFF_ENTRIES`.
    pub fn diff(&self) -> (Vec<ChangedFile>, usize) {
        let current = walk_tree(&self.root);

        let mut changed: Vec<ChangedFile> = Vec::new();

        // Files in current state — New or Modified.
        for (rel, cur_entry) in &current {
            match self.entries.get(rel) {
                None => {
                    // New file.
                    changed.push(ChangedFile {
                        path: rel.clone(),
                        kind: ChangeKind::New,
                        size: cur_entry.size,
                        old_size: 0,
                    });
                }
                Some(snap_entry) => {
                    // Modified if mtime or size changed.
                    if cur_entry.mtime != snap_entry.mtime || cur_entry.size != snap_entry.size {
                        changed.push(ChangedFile {
                            path: rel.clone(),
                            kind: ChangeKind::Modified,
                            size: cur_entry.size,
                            old_size: snap_entry.size,
                        });
                    }
                }
            }
        }

        // Files in snapshot but not current — Deleted.
        for rel in self.entries.keys() {
            if !current.contains_key(rel) {
                let old_size = self.entries[rel].size;
                changed.push(ChangedFile {
                    path: rel.clone(),
                    kind: ChangeKind::Deleted,
                    size: 0,
                    old_size,
                });
            }
        }

        let total = changed.len();

        // Sort: New → Modified → Deleted, each group alphabetically.
        changed.sort_by(|a, b| {
            kind_order(&a.kind)
                .cmp(&kind_order(&b.kind))
                .then_with(|| a.path.cmp(&b.path))
        });

        if changed.len() > MAX_DIFF_ENTRIES {
            changed.truncate(MAX_DIFF_ENTRIES);
        }

        (changed, total)
    }

    /// Reset this snapshot to the current filesystem state.
    pub fn reset(&mut self) {
        self.entries = walk_tree(&self.root);
        self.taken_at = SystemTime::now();
    }
}

/// Walk `root` recursively and return a map of `{relative_path → SnapshotEntry}`.
///
/// Hidden entries are included. Symlinks are not followed.
fn walk_tree(root: &Path) -> HashMap<PathBuf, SnapshotEntry> {
    let mut map = HashMap::new();
    walk_dir(root, root, &mut map);
    map
}

fn walk_dir(root: &Path, dir: &Path, map: &mut HashMap<PathBuf, SnapshotEntry>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.filter_map(|e| e.ok()) {
        let path = entry.path();
        // Skip symlinks to avoid cycles.
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        if ft.is_file() {
            if let Ok(meta) = entry.metadata() {
                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let size = meta.len();
                map.insert(rel, SnapshotEntry { mtime, size });
            }
        } else if ft.is_dir() {
            walk_dir(root, &path, map);
        }
    }
}

fn kind_order(k: &ChangeKind) -> u8 {
    match k {
        ChangeKind::New => 0,
        ChangeKind::Modified => 1,
        ChangeKind::Deleted => 2,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn make_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("trek_snap_{}_{}", name, std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &[u8]) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content).unwrap();
    }

    /// Given: a directory with two files
    /// When: capture() is called
    /// Then: both files appear in the snapshot entries
    #[test]
    fn capture_records_all_files() {
        let tmp = make_test_dir("capture_all");
        write_file(&tmp.join("a.txt"), b"hello");
        write_file(&tmp.join("b.txt"), b"world");
        let snap = SessionSnapshot::capture(&tmp);
        assert_eq!(snap.entries.len(), 2);
        assert!(snap.entries.contains_key(Path::new("a.txt")));
        assert!(snap.entries.contains_key(Path::new("b.txt")));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot of a directory with one file
    /// When: a new file is added and diff() is called
    /// Then: the new file appears as ChangeKind::New
    #[test]
    fn diff_detects_new_file() {
        let tmp = make_test_dir("diff_new");
        write_file(&tmp.join("existing.txt"), b"original");
        let snap = SessionSnapshot::capture(&tmp);

        write_file(&tmp.join("new.txt"), b"added later");
        let (changes, _) = snap.diff();
        let new_files: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::New)
            .collect();
        assert_eq!(new_files.len(), 1);
        assert_eq!(new_files[0].path, Path::new("new.txt"));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot of a directory with one file
    /// When: that file's content changes and diff() is called
    /// Then: the file appears as ChangeKind::Modified
    #[test]
    fn diff_detects_modified_file() {
        let tmp = make_test_dir("diff_modified");
        let file_path = tmp.join("file.txt");
        write_file(&file_path, b"original");
        let snap = SessionSnapshot::capture(&tmp);

        // Write different content (different size triggers detection).
        write_file(&file_path, b"changed content here");
        let (changes, _) = snap.diff();
        let modified: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0].path, Path::new("file.txt"));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot of a directory with one file
    /// When: that file is deleted and diff() is called
    /// Then: the file appears as ChangeKind::Deleted
    #[test]
    fn diff_detects_deleted_file() {
        let tmp = make_test_dir("diff_deleted");
        let file_path = tmp.join("gone.txt");
        write_file(&file_path, b"will be deleted");
        let snap = SessionSnapshot::capture(&tmp);

        fs::remove_file(&file_path).unwrap();
        let (changes, _) = snap.diff();
        let deleted: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::Deleted)
            .collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].path, Path::new("gone.txt"));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot of a directory with no changes
    /// When: diff() is called immediately
    /// Then: empty changes list is returned
    #[test]
    fn diff_unchanged_directory_returns_empty() {
        let tmp = make_test_dir("diff_unchanged");
        write_file(&tmp.join("stable.txt"), b"no changes");
        let snap = SessionSnapshot::capture(&tmp);
        let (changes, total) = snap.diff();
        assert!(
            changes.is_empty(),
            "expected no changes, got: {:?}",
            changes
        );
        assert_eq!(total, 0);
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot taken at some point
    /// When: reset() is called after adding a new file
    /// Then: the new file is no longer reported as New in the next diff
    #[test]
    fn reset_moves_baseline_to_current_state() {
        let tmp = make_test_dir("snap_reset");
        write_file(&tmp.join("original.txt"), b"exists");
        let mut snap = SessionSnapshot::capture(&tmp);

        write_file(&tmp.join("new.txt"), b"added");
        let (before_reset, _) = snap.diff();
        assert_eq!(before_reset.len(), 1, "should see 1 new file before reset");

        snap.reset();
        let (after_reset, _) = snap.diff();
        assert!(
            after_reset.is_empty(),
            "should see 0 changes after reset, got: {:?}",
            after_reset
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a snapshot and mixed changes
    /// When: diff() is called
    /// Then: results are ordered New → Modified → Deleted, each group alphabetical
    #[test]
    fn diff_results_are_sorted_new_modified_deleted() {
        let tmp = make_test_dir("diff_sorted");
        let to_modify = tmp.join("b_modify.txt");
        let to_delete = tmp.join("c_delete.txt");
        write_file(&to_modify, b"original");
        write_file(&to_delete, b"will go");
        let snap = SessionSnapshot::capture(&tmp);

        write_file(&tmp.join("a_new.txt"), b"brand new");
        write_file(&to_modify, b"changed content longer");
        fs::remove_file(&to_delete).unwrap();

        let (changes, _) = snap.diff();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].kind, ChangeKind::New);
        assert_eq!(changes[1].kind, ChangeKind::Modified);
        assert_eq!(changes[2].kind, ChangeKind::Deleted);
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a directory with a hidden file
    /// When: capture() is called
    /// Then: the hidden file is included in the snapshot
    #[test]
    fn capture_includes_hidden_files() {
        let tmp = make_test_dir("capture_hidden");
        write_file(&tmp.join(".hidden"), b"secret");
        write_file(&tmp.join("visible.txt"), b"open");
        let snap = SessionSnapshot::capture(&tmp);
        assert!(
            snap.entries.contains_key(Path::new(".hidden")),
            "hidden file must be in snapshot"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Given: a directory with a nested subdirectory
    /// When: capture() is called
    /// Then: files inside the subdirectory are included with relative paths
    #[test]
    fn capture_includes_nested_files() {
        let tmp = make_test_dir("capture_nested");
        fs::create_dir(tmp.join("sub")).unwrap();
        write_file(&tmp.join("sub").join("nested.rs"), b"fn foo() {}");
        let snap = SessionSnapshot::capture(&tmp);
        assert!(
            snap.entries.contains_key(Path::new("sub/nested.rs")),
            "nested file must be in snapshot"
        );
        let _ = fs::remove_dir_all(&tmp);
    }
}
