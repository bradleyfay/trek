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
| `l` / `→` / `Enter` | Enter the selected directory, or open the selected file |
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

### Marks (`` ` `` and `'`)

Marks let you pin up to 52 locations in memory and jump back to them instantly.

| Key | Action |
|-----|--------|
| `` ` `` + letter (`a`–`z`, `A`–`Z`) | Set a mark at the current directory |
| `'` + letter | Jump to a previously set mark |

Marks are session-only — they exist in memory and are cleared when Trek exits. For locations you want to return to across sessions, use bookmarks instead (see [File Operations — Bookmarks](file-operations.md#bookmarks)).

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
| `T` | Switch the size column between file size and last-modified timestamp |
| `N` | Switch the size column to show directory item counts instead of block size |
| `i` | Gitignore filter — hide gitignored entries; shows a yellow `[ignore]` badge in the path bar when active |
| `w` | Collapse or expand the right preview pane |

---

## Marks vs. Bookmarks

Trek has two separate systems for saving and returning to locations:

- **Marks** (`` ` `` / `'`) — session-only, stored in memory, cleared on exit. Fast to set and jump to during a working session.
- **Bookmarks** (`b` / `B`) — persistent, saved to `~/.local/share/trek/` and available across sessions. Use these for directories you return to regularly.

See [File Operations — Bookmarks](file-operations.md#bookmarks) for details on the bookmark keys.
