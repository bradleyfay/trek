use super::App;
use crate::app::session_snapshot::{ChangeKind, ChangedFile, SessionSnapshot};

impl App {
    /// Open the session summary pane.
    ///
    /// Takes a snapshot on first use (lazy). Recomputes the diff if the cache
    /// is stale (cleared by `reset_session_snapshot` or `refresh_session_summary`).
    pub fn open_session_summary(&mut self) {
        // Take the initial snapshot the first time.
        if self.session_snapshot.is_none() {
            self.session_snapshot = Some(SessionSnapshot::capture(&self.nav.cwd));
        }

        self.overlay.session_summary_mode = true;
        self.session_summary_selected = 0;

        if self.session_summary_cache.is_none() {
            self.recompute_session_summary();
        }
    }

    /// Close the session summary pane without navigating.
    pub fn close_session_summary(&mut self) {
        self.overlay.session_summary_mode = false;
    }

    /// Toggle the session summary pane open/closed.
    pub fn toggle_session_summary(&mut self) {
        if self.overlay.session_summary_mode {
            self.close_session_summary();
        } else {
            self.open_session_summary();
        }
    }

    /// Reset the session checkpoint to now and refresh the summary.
    pub fn reset_session_snapshot(&mut self) {
        let root = self.nav.cwd.clone();
        if let Some(ref mut snap) = self.session_snapshot {
            snap.root = root;
            snap.reset();
        } else {
            self.session_snapshot = Some(SessionSnapshot::capture(&self.nav.cwd));
        }
        self.session_summary_cache = None;
        self.session_summary_total = 0;
        self.session_summary_selected = 0;
        if self.overlay.session_summary_mode {
            self.recompute_session_summary();
        }
        self.status_message = Some("Session checkpoint reset".to_string());
    }

    /// Refresh the diff without resetting the checkpoint.
    pub fn refresh_session_summary(&mut self) {
        self.session_summary_cache = None;
        self.recompute_session_summary();
    }

    /// Move the summary cursor up.
    pub fn session_summary_move_up(&mut self) {
        if self.session_summary_selected > 0 {
            self.session_summary_selected -= 1;
        }
    }

    /// Move the summary cursor down.
    pub fn session_summary_move_down(&mut self) {
        let len = self
            .session_summary_cache
            .as_ref()
            .map(|c| c.len())
            .unwrap_or(0);
        if self.session_summary_selected + 1 < len {
            self.session_summary_selected += 1;
        }
    }

    /// Jump to the currently selected file in the normal tree view.
    ///
    /// Exits summary mode, navigates to the file's parent directory, and
    /// selects the file in the listing.
    pub fn session_summary_jump_to_selected(&mut self) {
        let Some(ref cache) = self.session_summary_cache else {
            self.close_session_summary();
            return;
        };
        let Some(entry) = cache.get(self.session_summary_selected).cloned() else {
            self.close_session_summary();
            return;
        };

        self.close_session_summary();

        // Resolve to an absolute path using the snapshot root.
        let root = self
            .session_snapshot
            .as_ref()
            .map(|s| s.root.clone())
            .unwrap_or_else(|| self.nav.cwd.clone());
        let abs_path = root.join(&entry.path);

        let parent = match abs_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let file_name = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());

        self.nav.filter_input.clear();
        self.nav.filter_mode = false;
        self.push_history(parent.clone());
        self.nav.cwd = parent;
        self.nav.selected = 0;
        self.nav.current_scroll = 0;
        self.load_dir();

        if let Some(name) = file_name {
            if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                self.nav.selected = idx;
                self.load_preview();
            }
        }
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn recompute_session_summary(&mut self) {
        if let Some(ref snap) = self.session_snapshot {
            let (changes, total) = snap.diff();
            self.session_summary_total = total;
            // Clamp selection to the new length.
            let len = changes.len();
            if self.session_summary_selected >= len && len > 0 {
                self.session_summary_selected = len - 1;
            }
            self.session_summary_cache = Some(changes);
        }
    }
}

/// Helper: count entries of a given kind in a change list.
pub fn count_by_kind(changes: &[ChangedFile], kind: &ChangeKind) -> usize {
    changes.iter().filter(|c| &c.kind == kind).count()
}
