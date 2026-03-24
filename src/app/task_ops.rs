use super::App;
use crate::app::task_manager::{PendingTask, TaskKind, TaskResult, TaskStatus};
use crate::ops::{self, ClipboardOp};
use std::path::PathBuf;
use std::sync::mpsc;

impl App {
    /// Toggle the task manager overlay open/closed.
    pub fn toggle_task_manager(&mut self) {
        self.task_manager_mode = !self.task_manager_mode;
    }

    /// Move the task manager cursor up.
    pub fn task_manager_move_up(&mut self) {
        self.task_manager.move_up();
    }

    /// Move the task manager cursor down.
    pub fn task_manager_move_down(&mut self) {
        self.task_manager.move_down();
    }

    /// Remove all completed tasks from the task manager.
    pub fn task_manager_clear_done(&mut self) {
        self.task_manager.clear_done();
    }

    /// Paste clipboard contents as a background task.
    ///
    /// Returns immediately — the actual copy/move runs on a background thread.
    /// Directory listing is refreshed when the task completes (via `check_task_rx`).
    pub fn paste_clipboard_async(&mut self) {
        let Some(clip) = self.clipboard.take() else {
            self.status_message = Some("Nothing in clipboard".to_string());
            return;
        };

        let dest_dir = self.cwd.clone();
        let op = clip.op;
        let paths = clip.paths.clone();

        // Collect (src, dst) pairs, checking for conflicts now (on the main thread).
        let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut skipped = 0usize;
        for src in &paths {
            let file_name = match src.file_name() {
                Some(n) => n,
                None => continue,
            };
            let dst = dest_dir.join(file_name);
            if dst.exists() && &dst != src {
                skipped += 1;
                continue;
            }
            if op == ClipboardOp::Cut && dst == *src {
                continue;
            }
            pairs.push((src.clone(), dst));
        }

        if pairs.is_empty() {
            if skipped > 0 {
                self.status_message =
                    Some(format!("{} skipped — destination already exists", skipped));
            } else {
                self.status_message = Some("Nothing to paste".to_string());
            }
            // Restore the clipboard for copy operations.
            if op == ClipboardOp::Copy {
                self.clipboard = Some(crate::ops::Clipboard { op, paths });
            }
            return;
        }

        // Build a short label for the task panel.
        let label = if pairs.len() == 1 {
            let name = pairs[0]
                .0
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("{} → {}", name, dest_dir.to_string_lossy())
        } else {
            format!("{} files → {}", pairs.len(), dest_dir.to_string_lossy())
        };

        let kind = match op {
            ClipboardOp::Copy => TaskKind::Copy,
            ClipboardOp::Cut => TaskKind::Move,
        };
        let task_id = self.task_manager.push(kind.clone(), label.clone());

        let verb = match kind {
            TaskKind::Copy => "Copying",
            TaskKind::Move => "Moving",
            _ => "Working on",
        };
        let short_label = if pairs.len() == 1 {
            pairs[0]
                .0
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            format!("{} files", pairs.len())
        };
        self.status_message = Some(format!("{} {}… (Ctrl+T to monitor)", verb, short_label));

        // For Copy: restore clipboard so repeated pastes work.
        if op == ClipboardOp::Copy {
            self.clipboard = Some(crate::ops::Clipboard { op, paths });
        }

        // Spawn background thread.
        let (tx, rx) = mpsc::channel::<TaskResult>();
        self.task_pending.push(PendingTask { rx });

        std::thread::spawn(move || {
            let mut done = 0usize;
            let mut errors: Vec<String> = Vec::new();
            for (src, dst) in &pairs {
                let result = match op {
                    ClipboardOp::Copy => ops::copy_path(src, dst),
                    ClipboardOp::Cut => ops::move_path(src, dst),
                };
                match result {
                    Ok(()) => done += 1,
                    Err(e) => errors.push(e.to_string()),
                }
            }
            let verb_past = match op {
                ClipboardOp::Copy => "Copied",
                ClipboardOp::Cut => "Moved",
            };
            let status = if let Some(err) = errors.first() {
                TaskStatus::Failed { error: err.clone() }
            } else {
                TaskStatus::Done {
                    summary: format!(
                        "{} {} item{}{}",
                        verb_past,
                        done,
                        if done == 1 { "" } else { "s" },
                        if skipped > 0 {
                            format!(" ({} skipped)", skipped)
                        } else {
                            String::new()
                        }
                    ),
                }
            };
            let _ = tx.send(TaskResult {
                task_id,
                status,
                refresh_dir: true,
            });
        });
    }

    /// Extract the currently pending archive as a background task.
    pub fn confirm_extract_async(&mut self) {
        let path = match self.pending_extract.take() {
            Some(p) => p,
            None => return,
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let dest_dir = self.cwd.clone();
        let label = format!("{} → {}", name, dest_dir.to_string_lossy());

        let task_id = self.task_manager.push(TaskKind::Extract, label);
        self.status_message = Some(format!("Extracting {}… (Ctrl+T to monitor)", name));

        let (tx, rx) = mpsc::channel::<TaskResult>();
        self.task_pending.push(PendingTask { rx });

        std::thread::spawn(move || {
            let status = match crate::archive::extract_archive(&path, &dest_dir) {
                Ok(()) => TaskStatus::Done {
                    summary: format!("Extracted: {}", name),
                },
                Err(e) => TaskStatus::Failed {
                    error: e.to_string(),
                },
            };
            let _ = tx.send(TaskResult {
                task_id,
                status,
                refresh_dir: true,
            });
        });
    }

    /// Poll all pending task receivers and apply any completed results.
    ///
    /// Must be called on every event-loop iteration.
    pub fn check_task_rx(&mut self) {
        let mut completed: Vec<(u64, TaskStatus, bool)> = Vec::new();
        self.task_pending.retain(|pt| match pt.rx.try_recv() {
            Ok(result) => {
                completed.push((result.task_id, result.status, result.refresh_dir));
                false
            }
            Err(mpsc::TryRecvError::Empty) => true,
            Err(mpsc::TryRecvError::Disconnected) => false,
        });

        for (id, status, refresh) in completed {
            // Update status message for the most-recently-completed task.
            let summary = match &status {
                TaskStatus::Done { summary } => summary.clone(),
                TaskStatus::Failed { error } => format!("Error: {}", error),
                TaskStatus::Running => unreachable!(),
            };
            self.status_message = Some(summary);
            self.task_manager.update(id, status);
            if refresh {
                self.load_dir();
                self.git_status = crate::git::GitStatus::load(&self.cwd);
            }
        }
    }
}
