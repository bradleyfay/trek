//! Filesystem watcher for auto-refreshing the directory listing.
//!
//! Wraps `notify-debouncer-mini` to provide OS-native filesystem events
//! (FSEvents on macOS, inotify on Linux) with a 150 ms debounce window.
//!
//! The watcher is always-on — Trek starts watching the current directory
//! automatically and updates the watch target whenever the directory changes.
//! Users can toggle it off/on with `I` if they prefer manual refresh.

use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Watches a single directory for filesystem changes.
///
/// The `rx` channel receives debounced event batches. Poll `rx.try_recv()`
/// in the event loop — it is non-blocking and returns immediately when no
/// events are pending.
///
/// Dropping this struct cancels the watch automatically.
pub struct DirWatcher {
    /// Receive end of the debounced event channel.
    pub rx: Receiver<Vec<DebouncedEvent>>,
    // Keep the debouncer alive; dropping it cancels the underlying OS watch.
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl DirWatcher {
    /// Start watching `dir` for changes.
    ///
    /// Returns `None` if the OS watcher fails to initialise (e.g. inotify
    /// limit reached, read-only filesystem). Trek degrades gracefully to
    /// manual refresh (`R`) when this happens.
    pub fn new(dir: &Path) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_millis(150), move |res| {
            if let Ok(events) = res {
                let _ = tx.send(events);
            }
        })
        .ok()?;

        debouncer
            .watcher()
            .watch(dir, notify::RecursiveMode::NonRecursive)
            .ok()?;

        Some(DirWatcher {
            rx,
            _debouncer: debouncer,
        })
    }
}
