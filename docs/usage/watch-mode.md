# Watch Mode

Watch mode keeps the Trek file listing current without manual refreshes. When active, Trek detects changes to the current directory and reloads automatically.

---

## Toggling Watch Mode (`I`)

Press `I` (uppercase) to enable watch mode. Press `I` again to disable it.

| State | Status bar message |
|-------|--------------------|
| Enabled | `"Watch mode ON — listing auto-refreshes on changes"` |
| Disabled | `"Watch mode OFF"` |

While watch mode is active, a cyan `[watch]` badge appears in the path bar as a persistent indicator.

---

## How It Works

Trek polls the current directory's modification time (`mtime`) at a 500ms interval. When a change is detected, it reloads the directory listing and attempts to restore the cursor to the previously selected entry by name, so the cursor does not jump to an unexpected position after a refresh.

The poll interval is short enough to catch rapid changes (build output, log rotation) without busy-waiting or noticeable CPU overhead.

---

## Use Cases

Watch mode is useful any time the contents of a directory change without your direct involvement:

- **Build output** — monitor a `dist/` or `target/` directory for newly built artifacts
- **Log directories** — watch for new log files written by a running process
- **Downloads** — keep a downloads folder current as files arrive
- **Generated files** — observe test output, code generation, or data pipeline results as they are written

---

## Limitations

Watch mode monitors the current directory only. Navigating to a different directory while watch mode is active will begin monitoring the new directory. Watch mode does not recursively watch subdirectories.
