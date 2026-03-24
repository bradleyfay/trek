//! Live change feed — records filesystem events from the recursive watcher
//! and exposes them for display in the preview pane area.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Maximum events retained in the feed. Oldest are evicted when the buffer is full.
pub const MAX_FEED_EVENTS: usize = 500;

/// Path segments that are suppressed from the feed (build artifacts, VCS dirs, etc.).
pub const SUPPRESSED_SEGMENTS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".next",
    "dist",
    "build",
];

/// The kind of filesystem event recorded in the feed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeedEventKind {
    Created,
    Modified,
    Deleted,
}

impl FeedEventKind {
    /// Short symbol rendered in the feed pane (one character).
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Created => "+",
            Self::Modified => "~",
            Self::Deleted => "✕",
        }
    }
}

/// One event recorded in the live change feed.
pub struct FeedEvent {
    pub path: PathBuf,
    pub kind: FeedEventKind,
    /// Wall-clock instant when the event was pushed into the feed.
    pub recorded_at: Instant,
}

/// Holds the ring-buffer of recent filesystem events and cursor state.
pub struct ChangeFeed {
    /// Recent events, most-recent first (index 0 = newest).
    pub events: VecDeque<FeedEvent>,
    /// Index into `events` of the highlighted row.
    pub selected: usize,
    /// Hard cap; oldest events are evicted once this is reached.
    pub max_events: usize,
}

impl ChangeFeed {
    pub fn new() -> Self {
        ChangeFeed {
            events: VecDeque::new(),
            selected: 0,
            max_events: MAX_FEED_EVENTS,
        }
    }

    /// Push a new event to the front (most-recent-first).
    ///
    /// Evicts the oldest event when the buffer is full. The cursor is shifted
    /// so the same event remains highlighted after the insert.
    pub fn push(&mut self, event: FeedEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_back();
            // If cursor was beyond the new end, clamp it.
            if self.selected >= self.events.len() && !self.events.is_empty() {
                self.selected = self.events.len() - 1;
            }
        }
        self.events.push_front(event);
        // Shift cursor down to stay on the same event (new event inserted at front).
        if !self.events.is_empty() && self.selected + 1 < self.events.len() {
            self.selected += 1;
        }
    }

    /// Clear all events and reset the cursor to zero.
    pub fn clear(&mut self) {
        self.events.clear();
        self.selected = 0;
    }

    /// Move cursor toward the front (newer events), clamped at 0.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move cursor toward the back (older events), clamped at the last event.
    pub fn move_down(&mut self) {
        if !self.events.is_empty() && self.selected < self.events.len() - 1 {
            self.selected += 1;
        }
    }

    /// Return the path of the currently highlighted event, if any.
    pub fn selected_path(&self) -> Option<&Path> {
        self.events.get(self.selected).map(|e| e.path.as_path())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(path: &str, kind: FeedEventKind) -> FeedEvent {
        FeedEvent {
            path: PathBuf::from(path),
            kind,
            recorded_at: Instant::now(),
        }
    }

    /// Given: a ChangeFeed at max capacity
    /// When: a new event is pushed
    /// Then: the oldest event (back of VecDeque) is evicted and length stays at max
    #[test]
    fn push_evicts_oldest_when_full() {
        let mut feed = ChangeFeed::new();
        feed.max_events = 3;

        feed.push(make_event("a", FeedEventKind::Created));
        feed.push(make_event("b", FeedEventKind::Modified));
        feed.push(make_event("c", FeedEventKind::Modified));
        assert_eq!(feed.events.len(), 3);

        // "a" is now the oldest (back).
        feed.push(make_event("d", FeedEventKind::Deleted));
        assert_eq!(feed.events.len(), 3, "should stay at max_events");
        // "d" is at front, "a" should have been evicted.
        assert!(
            feed.events.iter().all(|e| e.path != PathBuf::from("a")),
            "oldest event 'a' should be evicted"
        );
        assert_eq!(feed.events[0].path, PathBuf::from("d"));
    }

    /// Given: a ChangeFeed with several events
    /// When: clear() is called
    /// Then: events is empty and selected resets to 0
    #[test]
    fn clear_resets_buffer_and_cursor() {
        let mut feed = ChangeFeed::new();
        feed.push(make_event("x", FeedEventKind::Created));
        feed.push(make_event("y", FeedEventKind::Modified));
        feed.selected = 1;

        feed.clear();

        assert!(feed.events.is_empty(), "events should be empty after clear");
        assert_eq!(feed.selected, 0, "cursor should reset to 0 after clear");
    }

    /// Given: a ChangeFeed with cursor at 0
    /// When: move_up() is called
    /// Then: cursor stays at 0 (no underflow)
    #[test]
    fn move_up_clamps_at_zero() {
        let mut feed = ChangeFeed::new();
        feed.push(make_event("f", FeedEventKind::Modified));
        feed.selected = 0;

        feed.move_up();

        assert_eq!(feed.selected, 0, "cursor should not go below 0");
    }

    /// Given: a ChangeFeed with cursor at the last event
    /// When: move_down() is called
    /// Then: cursor stays at the last index (no overflow)
    #[test]
    fn move_down_clamps_at_end() {
        let mut feed = ChangeFeed::new();
        feed.push(make_event("a", FeedEventKind::Created));
        feed.push(make_event("b", FeedEventKind::Modified));
        let last = feed.events.len() - 1;
        feed.selected = last;

        feed.move_down();

        assert_eq!(
            feed.selected, last,
            "cursor should not exceed last event index"
        );
    }

    /// Given: a ChangeFeed with one event
    /// When: selected_path() is called
    /// Then: returns the path of the highlighted event
    #[test]
    fn selected_path_returns_highlighted_event_path() {
        let mut feed = ChangeFeed::new();
        feed.push(make_event("src/main.rs", FeedEventKind::Modified));

        let path = feed.selected_path();
        assert!(path.is_some());
        assert_eq!(path.unwrap(), Path::new("src/main.rs"));
    }

    /// Given: an empty ChangeFeed
    /// When: selected_path() is called
    /// Then: returns None
    #[test]
    fn selected_path_returns_none_when_empty() {
        let feed = ChangeFeed::new();
        assert!(feed.selected_path().is_none());
    }
}
