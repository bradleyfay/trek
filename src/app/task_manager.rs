use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Instant;

/// The kind of file operation a background task is performing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskKind {
    Copy,
    Move,
    Extract,
}

impl TaskKind {
    pub fn label(&self) -> &'static str {
        match self {
            TaskKind::Copy => "Copy",
            TaskKind::Move => "Move",
            TaskKind::Extract => "Extract",
        }
    }
}

/// The current status of a background file task.
#[derive(Clone, Debug)]
pub enum TaskStatus {
    /// The operation is in progress on a background thread.
    Running,
    /// The operation completed successfully.
    Done {
        /// Human-readable summary, e.g. "Copied 3 files".
        summary: String,
    },
    /// The operation failed.
    Failed {
        /// The first error message from the operation.
        error: String,
    },
}

/// A single background file operation tracked by the task manager.
#[derive(Clone, Debug)]
pub struct FileTask {
    /// Unique monotonic ID assigned at creation.
    pub id: u64,
    pub kind: TaskKind,
    /// Short display label, e.g. "config.toml" or "5 files → /tmp".
    pub label: String,
    pub status: TaskStatus,
    pub started_at: Instant,
}

impl FileTask {
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }
}

/// Result message sent from a background thread back to the main thread.
pub struct TaskResult {
    pub task_id: u64,
    pub status: TaskStatus,
    /// True when the task mutated the filesystem and the directory listing
    /// should be refreshed.
    pub refresh_dir: bool,
}

/// An in-flight background task: its ID and the receiver end of its channel.
pub struct PendingTask {
    pub rx: mpsc::Receiver<TaskResult>,
}

/// Manages background file tasks.
///
/// Holds the ordered list of tasks (most recent first) and tracks which row
/// the user has highlighted in the overlay.
pub struct TaskManager {
    /// Tasks, most recent first.  Capped at MAX_TASKS.
    pub tasks: VecDeque<FileTask>,
    /// Index into `tasks` of the highlighted row (0 = most recent).
    pub selected: usize,
    next_id: u64,
}

/// Maximum number of tasks kept in the panel (oldest evicted when full).
const MAX_TASKS: usize = 100;

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            selected: 0,
            next_id: 1,
        }
    }

    /// Register a new Running task and return its ID.
    pub fn push(&mut self, kind: TaskKind, label: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push_front(FileTask {
            id,
            kind,
            label,
            status: TaskStatus::Running,
            started_at: Instant::now(),
        });
        if self.tasks.len() > MAX_TASKS {
            self.tasks.pop_back();
        }
        // Keep cursor in bounds.
        self.selected = self.selected.min(self.tasks.len().saturating_sub(1));
        id
    }

    /// Update the status of the task with `id`.
    ///
    /// No-op if the task is not found (already evicted).
    pub fn update(&mut self, id: u64, status: TaskStatus) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = status;
        }
    }

    /// Move the highlight cursor up (toward more-recent tasks).
    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Move the highlight cursor down (toward older tasks).
    pub fn move_down(&mut self) {
        if !self.tasks.is_empty() {
            self.selected = (self.selected + 1).min(self.tasks.len() - 1);
        }
    }

    /// Remove all completed tasks (Done and Failed).
    pub fn clear_done(&mut self) {
        self.tasks.retain(|t| t.is_running());
        self.selected = self.selected.min(self.tasks.len().saturating_sub(1));
    }

    /// True when at least one task is currently running.
    pub fn has_running(&self) -> bool {
        self.tasks.iter().any(|t| t.is_running())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given: an empty TaskManager
    /// When: a task is pushed
    /// Then: it appears at position 0 with Running status
    #[test]
    fn push_adds_task_at_front() {
        let mut tm = TaskManager::new();
        let id = tm.push(TaskKind::Copy, "file.txt".to_string());
        assert_eq!(tm.tasks.len(), 1);
        assert_eq!(tm.tasks[0].id, id);
        assert!(tm.tasks[0].is_running());
    }

    /// Given: a running task
    /// When: update is called with Done status
    /// Then: the task's status becomes Done
    #[test]
    fn update_changes_task_status() {
        let mut tm = TaskManager::new();
        let id = tm.push(TaskKind::Move, "dir/".to_string());
        tm.update(
            id,
            TaskStatus::Done {
                summary: "Moved 1 item".to_string(),
            },
        );
        assert!(!tm.tasks[0].is_running());
    }

    /// Given: two tasks (one running, one done)
    /// When: clear_done is called
    /// Then: only the running task remains
    #[test]
    fn clear_done_removes_completed_tasks() {
        let mut tm = TaskManager::new();
        let id1 = tm.push(TaskKind::Copy, "a.txt".to_string());
        let id2 = tm.push(TaskKind::Move, "b.txt".to_string());
        tm.update(
            id1,
            TaskStatus::Done {
                summary: "ok".to_string(),
            },
        );
        tm.clear_done();
        assert_eq!(tm.tasks.len(), 1);
        assert_eq!(tm.tasks[0].id, id2);
    }

    /// Given: a TaskManager with 3 tasks
    /// When: move_down and move_up are called
    /// Then: selected is clamped at boundaries
    #[test]
    fn move_cursor_clamps_at_boundaries() {
        let mut tm = TaskManager::new();
        tm.push(TaskKind::Copy, "a".to_string());
        tm.push(TaskKind::Copy, "b".to_string());
        tm.push(TaskKind::Copy, "c".to_string());
        assert_eq!(tm.selected, 0);
        tm.move_up(); // at top already
        assert_eq!(tm.selected, 0);
        tm.move_down();
        assert_eq!(tm.selected, 1);
        tm.move_down();
        assert_eq!(tm.selected, 2);
        tm.move_down(); // at bottom
        assert_eq!(tm.selected, 2);
    }

    /// Given: a TaskManager with one running and one failed task
    /// When: has_running is called
    /// Then: returns true
    #[test]
    fn has_running_returns_true_when_any_task_is_running() {
        let mut tm = TaskManager::new();
        let id = tm.push(TaskKind::Extract, "archive.zip".to_string());
        tm.push(TaskKind::Copy, "other.txt".to_string());
        tm.update(
            id,
            TaskStatus::Failed {
                error: "not found".to_string(),
            },
        );
        assert!(tm.has_running());
    }

    /// Given: a TaskManager with only done/failed tasks
    /// When: has_running is called
    /// Then: returns false
    #[test]
    fn has_running_returns_false_when_no_task_is_running() {
        let mut tm = TaskManager::new();
        let id = tm.push(TaskKind::Copy, "f.txt".to_string());
        tm.update(
            id,
            TaskStatus::Done {
                summary: "ok".to_string(),
            },
        );
        assert!(!tm.has_running());
    }

    /// Given: a TaskManager filled to MAX_TASKS
    /// When: one more task is pushed
    /// Then: the total count stays at MAX_TASKS (oldest evicted)
    #[test]
    fn push_evicts_oldest_when_full() {
        let mut tm = TaskManager::new();
        for i in 0..=MAX_TASKS {
            tm.push(TaskKind::Copy, format!("file_{i}"));
        }
        assert_eq!(tm.tasks.len(), MAX_TASKS);
    }
}
