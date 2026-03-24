//! App methods for the live change feed (F).

use super::{change_feed::FeedEvent, App};

impl App {
    /// Toggle the change feed pane open/closed.
    pub fn toggle_change_feed(&mut self) {
        self.change_feed_mode = !self.change_feed_mode;
    }

    /// Move the feed cursor toward newer events (up in the list).
    pub fn change_feed_move_up(&mut self) {
        self.change_feed.move_up();
    }

    /// Move the feed cursor toward older events (down in the list).
    pub fn change_feed_move_down(&mut self) {
        self.change_feed.move_down();
    }

    /// Clear all events from the change feed buffer.
    pub fn clear_change_feed(&mut self) {
        self.change_feed.clear();
    }

    /// Navigate to the file or directory under the feed cursor.
    ///
    /// Closes the feed, navigates to the entry's parent directory, and selects
    /// the entry by name — identical to how `jump_to_find_result` works.
    pub fn jump_to_feed_entry(&mut self) {
        let path = match self.change_feed.selected_path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        // Close the feed before navigating so the UI returns to normal mode.
        self.change_feed_mode = false;

        let target_dir = if path.is_dir() {
            path.clone()
        } else {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(path.clone())
        };

        self.cwd = target_dir;
        self.load_dir();

        // Select the entry by name within the newly loaded directory.
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
            self.selected = idx;
            self.load_preview();
        }
    }

    /// Poll the recursive watcher and push any pending events into the feed.
    ///
    /// Called from the event loop on each iteration so the feed stays live
    /// while the user is browsing or while the feed overlay is open.
    pub fn check_recursive_watcher(&mut self) {
        use crate::app::change_feed::{FeedEventKind, SUPPRESSED_SEGMENTS};
        use notify::EventKind;

        // Drain all pending events into a local Vec first so the borrow of
        // `self.recursive_watcher` ends before we mutate `self.change_feed`.
        let raw_events: Vec<notify::Event> = match self.recursive_watcher.as_ref() {
            Some(w) => {
                let mut buf = Vec::new();
                while let Ok(Ok(ev)) = w.rx.try_recv() {
                    buf.push(ev);
                }
                buf
            }
            None => return,
        };

        for event in raw_events {
            let kind = match event.kind {
                EventKind::Create(_) => FeedEventKind::Created,
                EventKind::Modify(_) => FeedEventKind::Modified,
                EventKind::Remove(_) => FeedEventKind::Deleted,
                _ => continue, // skip access, meta, etc.
            };

            for path in event.paths {
                // Suppress build artifacts and VCS directories.
                if path.components().any(|c| {
                    SUPPRESSED_SEGMENTS
                        .iter()
                        .any(|seg| c.as_os_str() == std::ffi::OsStr::new(seg))
                }) {
                    continue;
                }

                self.change_feed.push(FeedEvent {
                    path,
                    kind,
                    recorded_at: std::time::Instant::now(),
                });
            }
        }
    }
}
