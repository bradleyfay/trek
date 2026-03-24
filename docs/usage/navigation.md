# Navigation

Trek's three-pane layout keeps you oriented at all times: the left pane shows the parent directory, the center pane shows the current directory, and the right pane previews the selected entry. Navigation moves you through that structure using keyboard shortcuts, mouse clicks, or both.

---

## Directory Navigation

Move the cursor and change directories with these keys:

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `h` / `←` | Go to parent directory |
| `l` / `→` / `Enter` | Enter the selected directory; open the selected file in a new cmux tab |
| `g` | Jump to top of listing |
| `G` | Jump to bottom of listing |
| `~` | Go to your home directory |
| `.` | Toggle hidden files (dotfiles) |

Arrow keys and vim-style `hjkl` are interchangeable throughout Trek. You do not need to know vim to use Trek — the arrow keys work everywhere.

---

## Jump Navigation

For longer jumps, Trek offers several ways to move to a specific location without scrolling:

### Path bar (`e`)

Press `e` to open the path jump bar. Type any path — absolute (`/usr/local/bin`), relative (`../../other-project`), or home-relative (`~/Documents`) — and press `Enter` to navigate there directly. Press `Esc` to cancel.

The jump bar supports Tab completion as you type:

| Situation | What `Tab` does |
|-----------|-----------------|
| Single match | Completes the entry name; appends `/` if it is a directory |
| Multiple matches | Advances to the longest common prefix shared by all matches |
| No matches | No-op (silent) |

A `~` prefix is expanded to your home directory before completion is applied. The hint line inside the jump bar reads `Tab=complete  Enter=go  Esc=cancel`.

### Marks (`` ` `` and `'`)

Marks let you pin up to 52 locations in memory and jump back to them instantly.

| Key | Action |
|-----|--------|
| `` ` `` + letter (`a`–`z`, `A`–`Z`) | Set a mark at the current directory |
| `'` + letter | Jump to a previously set mark |

Marks are saved as part of Trek's session state. When you quit with `q` or `Q` and relaunch Trek without arguments, your marks are restored automatically. For named, always-available locations, use bookmarks instead (see [File Operations — Bookmarks](file-operations.md#bookmarks)).

### Frecency jump list (`z`)

Press `z` to open the frecency jump list: an overlay showing directories you have visited recently, ranked by a combination of visit frequency and recency. Type to filter the list, then press `Enter` to jump to the selected entry. Press `Esc` to close without navigating.

### History navigation

Trek maintains a full navigation history for the current session:

| Key | Action |
|-----|--------|
| `Ctrl+O` | Navigate back in history |
| `Ctrl+I` | Navigate forward in history |

---

## Preview Scrolling

The right pane previews the selected file. Scroll it without moving the cursor:

| Key | Action |
|-----|--------|
| `[` | Scroll preview up 5 lines |
| `]` | Scroll preview down 5 lines |
| Mouse scroll wheel | Scroll preview 3 lines per event |

---

## View Toggles

These keys change what the center pane shows without navigating anywhere:

| Key | Toggle |
|-----|--------|
| `.` | Show/hide hidden files (dotfiles) |
| `S` | Cycle sort mode (name → size → modified → type) |
| `s` | Toggle sort order (ascending ↔ descending) |
| `T` | Switch the size column between file size and last-modified timestamp |
| `N` | Switch the size column to show directory item counts instead of block size |
| `i` | Gitignore filter — hide gitignored entries; shows a yellow `[ignore]` badge in the path bar when active |
| `w` | Collapse or expand the right preview pane |

---

## Marks vs. Bookmarks

Trek has two separate systems for saving and returning to locations:

- **Marks** (`` ` `` / `'`) — saved as part of Trek's session state on a clean quit (`q` / `Q`) and restored the next time Trek launches without arguments. Fast to set and jump to during a working session.
- **Bookmarks** (`b` / `B`) — always-persistent, saved to `~/.local/share/trek/` and available regardless of how Trek was launched. Use these for directories you want permanently available.

See [File Operations — Bookmarks](file-operations.md#bookmarks) for details on the bookmark keys.

---

## Session Restore

Trek saves your view state when you quit with `q` or `Q` and restores it the next time you launch without arguments. This means you return to the same context you left — directory, position, and display settings — without any extra steps.

The following state is saved and restored:

| State | Description |
|-------|-------------|
| Current directory | Where you were browsing when you quit |
| Selected entry | The entry your cursor was on |
| Marks | All marks set during the session (`` ` `` / `'`) |
| Hidden files toggle | Whether `.` was active (showing dotfiles) |
| Sort mode | Which sort column was active (`S` cycles: name → size → modified → type) |
| Sort order | Whether the listing was ascending or descending (`s` toggles) |

**Rules:**

- `trek /path` — always opens at the specified path and ignores saved session state.
- Session state is only written on a clean quit (`q` or `Q`). Closing the terminal window without quitting does not save state.
- If the saved directory has been deleted or is unavailable, Trek falls back to the current working directory silently.
- Unknown keys in the session file are silently ignored, so session files from older Trek versions are safe to keep.

**Session file location:**

Trek stores its session file at `$XDG_DATA_HOME/trek/session`, which defaults to `~/.local/share/trek/session`.
