# Watch Mode

Trek automatically watches the current directory for filesystem changes and refreshes the listing within ~150 ms — no configuration required. Watch mode is always on by default.

---

## Toggling Watch Mode (`I`)

Press `I` (uppercase) to disable watch mode. Press `I` again to re-enable it.

| State | Status bar message |
|-------|--------------------|
| Enabled (default) | `"Watch mode ON — listing auto-refreshes on changes"` |
| Disabled | `"Watch mode OFF"` |

When watch mode is active, a cyan `[watch]` badge appears in the path bar. When you disable it, the badge disappears and the listing no longer auto-refreshes. Use `R` to refresh the git status overlay at any time.

---

## How It Works

Trek uses OS-native filesystem events via the `notify` crate — FSEvents on macOS and inotify on Linux. This means changes appear within ~150 ms of any file creation, deletion, rename, or modification in the current directory, without polling.

A debounce window of 150 ms coalesces rapid bursts (such as `git checkout` touching many files) into a single reload. When a reload occurs, Trek attempts to restore the cursor to the previously selected entry by name so the cursor does not jump to an unexpected position.

The watcher updates automatically when you navigate to a new directory.

**Graceful degradation:** if the OS watcher fails to start (for example, due to an inotify instance limit or a read-only filesystem), Trek continues working normally and falls back to manual `R` refresh.

---

## Use Cases

Because watch mode is always on, the file tree stays current without any action on your part. This is especially useful when:

- **Build output** — artifacts appear in `dist/` or `target/` as they are built
- **Log directories** — new log files written by a running process appear immediately
- **Downloads** — files appear in a downloads folder as they arrive
- **Generated files** — test output, code generation, or data pipeline results are visible as they are written

If you find the auto-refresh distracting in a particular session, press `I` to turn it off.

---

## Limitations

Watch mode monitors the current directory only. Navigating to a different directory automatically moves the watcher to the new location. Watch mode does not recursively watch subdirectories.
