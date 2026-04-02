use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git status of a single file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Unstaged modifications in working tree.
    Modified,
    /// Changes staged in the index only.
    Staged,
    /// Both staged changes and additional working-tree modifications.
    StagedModified,
    /// New file not tracked by git.
    Untracked,
    /// Merge conflict / unmerged.
    Conflict,
    /// Deleted in index or working tree.
    Deleted,
}

/// Cached git repository status for the current navigation session.
pub struct GitStatus {
    /// Per-file status, keyed by absolute path.
    pub file_statuses: HashMap<PathBuf, FileStatus>,
    /// Current branch name, or "HEAD:<hash>" when detached.
    pub branch: Option<String>,
    /// Absolute path to the repository root (output of `git rev-parse --show-toplevel`).
    pub repo_root: PathBuf,
    /// Directories that contain at least one changed file (transitively).
    dirty_dirs: HashSet<PathBuf>,
}

impl GitStatus {
    /// Load git status for the repository containing `dir`.
    ///
    /// Returns `None` if `dir` is not inside a git repository or git is
    /// not available.
    pub fn load(dir: &Path) -> Option<Self> {
        let root_out = run_git(dir, &["rev-parse", "--show-toplevel"])?;
        let repo_root = PathBuf::from(root_out.trim());

        let branch = run_git(dir, &["branch", "--show-current"])
            .map(|b| b.trim().to_string())
            .and_then(|b| {
                if b.is_empty() {
                    // Detached HEAD — show short commit hash.
                    run_git(dir, &["rev-parse", "--short", "HEAD"])
                        .map(|h| format!("HEAD:{}", h.trim()))
                } else {
                    Some(b)
                }
            });

        let status_out = run_git(dir, &["status", "--porcelain=v1", "-u"])?;
        let mut file_statuses = HashMap::new();

        for line in status_out.lines() {
            if line.len() < 3 {
                continue;
            }
            let idx_char = line.chars().nth(0).unwrap_or(' ');
            let wt_char = line.chars().nth(1).unwrap_or(' ');
            let file_part = &line[3..];
            // Porcelain v1 shows renames as "old -> new".
            let filename = if file_part.contains(" -> ") {
                file_part.split(" -> ").last().unwrap_or(file_part)
            } else {
                file_part
            };
            let abs_path = repo_root.join(filename.trim());
            file_statuses.insert(abs_path, classify(idx_char, wt_char));
        }

        // Build set of all directories that transitively contain a changed file.
        // Each path walks up until it either hits the repo root (already in set) or
        // reaches a directory already marked dirty (which means all ancestors are too).
        let mut dirty_dirs: HashSet<PathBuf> = HashSet::new();
        for path in file_statuses.keys() {
            let mut cur = path.parent();
            while let Some(dir) = cur {
                if !dirty_dirs.insert(dir.to_path_buf()) {
                    break; // dir and all its ancestors are already marked
                }
                if dir == repo_root {
                    break;
                }
                cur = dir.parent();
            }
        }

        Some(GitStatus {
            file_statuses,
            branch,
            repo_root,
            dirty_dirs,
        })
    }

    /// Return the git status for a specific file path.
    pub fn for_path(&self, path: &Path) -> Option<FileStatus> {
        self.file_statuses.get(path).copied()
    }

    /// Return true if `dir` (or any descendant) contains a changed file.
    pub fn subtree_dirty(&self, dir: &Path) -> bool {
        self.dirty_dirs.contains(dir)
    }
}

/// Classify a porcelain v1 two-character status code into a `FileStatus`.
fn classify(idx: char, wt: char) -> FileStatus {
    match (idx, wt) {
        // Conflict / unmerged states.
        ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => FileStatus::Conflict,
        // Deleted in either index or working tree (but not untracked).
        (i, w) if (i == 'D' || w == 'D') && i != '?' => FileStatus::Deleted,
        // Completely untracked.
        ('?', '?') => FileStatus::Untracked,
        // Both index and working-tree changes.
        (i, w) if i != ' ' && i != '?' && w != ' ' && w != '?' => FileStatus::StagedModified,
        // Index-only changes (staged).
        (i, _) if i != ' ' && i != '?' => FileStatus::Staged,
        // Default: working-tree modification.
        _ => FileStatus::Modified,
    }
}

/// Return the set of entry names (not full paths) in `dir` that are gitignored.
///
/// Uses `git ls-files --others --ignored --exclude-standard --directory`
/// scoped to `dir`. Returns an empty set if `dir` is not in a git repo,
/// has no ignored entries, or git is unavailable (silent degradation).
///
/// The `--directory` flag makes git report ignored directories as a single
/// `dirname/` entry rather than listing every file inside, which is exactly
/// what is needed for filtering the directory listing.
pub fn load_ignored(dir: &Path) -> HashSet<String> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
        ])
        .current_dir(dir)
        .output();

    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => return HashSet::new(),
    };

    String::from_utf8_lossy(&out.stdout)
        .lines()
        // git appends '/' to directory entries; strip it for name comparison.
        .map(|line| line.trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Result type returned by the async git-status background thread.
pub struct GitStatusAsyncResult {
    /// Current repository status, or `None` outside a git repo.
    pub status: Option<GitStatus>,
    /// Gitignored entry names for the directory, or `None` when
    /// `hide_gitignored` was false so the load was skipped.
    pub ignored_names: Option<HashSet<String>>,
}

/// Run a git command with `-C <dir>` and return stdout on success.
fn run_git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}
