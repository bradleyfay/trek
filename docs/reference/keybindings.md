# Keybinding Reference

Trek v0.55.0

This page lists every keybinding available in Trek, organized by category.
If you can't find what you need here, press `:` to open the command palette and
type any part of the action name to find and run it.

---

## Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `h` / `←` | Go to parent directory |
| `l` / `→` / `Enter` | Enter directory; open file in a new cmux tab (see [cmux Integration](#cmux-integration)) |
| `g` | Jump to top of listing |
| `G` | Jump to bottom of listing |
| `~` | Go to home directory |
| `e` | Open path jump bar (type any path; press `Tab` to complete) |
| `` ` `` + letter | Set session mark at current directory |
| `'` + letter | Jump to session mark |
| `z` | Open frecency jump list |
| `Ctrl+O` | Navigate back in history |
| `Ctrl+I` | Navigate forward in history |
| `[` | Scroll preview pane up 5 lines |
| `]` | Scroll preview pane down 5 lines |

Session marks are saved when you quit with `q` or `Q` and restored the next
time Trek launches without arguments. For always-available bookmarks, see the
Bookmarks section below.

---

## cmux Integration

Pressing `l`, `→`, or `Enter` on a **file** opens it in a new cmux surface. Trek routes the file based on its type:

| File type | Opens with |
|-----------|-----------|
| HTML (`.html`, `.htm`) | System default opener (`open` / `xdg-open`) |
| Images (`.png`, `.jpg`, `.gif`, etc.) | System default opener |
| PDFs (`.pdf`) | System default opener |
| All other text / code files | `$EDITOR` in a new terminal surface |

**Mouse actions also trigger cmux routing:**

| Mouse action | Effect |
|--------------|--------|
| Right-click | Select the entry and open it in a new cmux tab (same routing as `l` / `Enter`) |
| Double-click | Open the file in a new cmux pane split to the right (`cmux new-pane --direction right`); falls back to the system opener for images, HTML, and PDFs |

When Trek is not running inside cmux, pressing `l`, `→`, or `Enter` on a file, right-clicking a file, or double-clicking a file all fall back gracefully and show a hint in the status bar. Use `o` to open in `$EDITOR` directly, or `O` to open with the system default, without requiring cmux.

To copy a file path without opening the file, use `y` (relative path) or `Y` (absolute path). The `l/→/Enter` keys no longer yank to the clipboard.

---

## File Operations

| Key | Action |
|-----|--------|
| `o` | Open file in terminal editor (`$EDITOR`) |
| `O` | Open file with system default |
| `M` | Create new directory |
| `t` | Create new empty file (touch) |
| `n` / `F2` | Quick rename current entry |
| `W` | Duplicate current entry in place |
| `L` | Create symlink to current entry |
| `c` | Copy to clipboard |
| `C` | Copy all selected entries to clipboard |
| `x` | Cut to clipboard |
| `X` | Cut all selected entries to clipboard |
| `p` | Paste from clipboard |
| `F` | Open clipboard inspector |
| `d` | Delete / trash current entry |

---

## Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle selection on current entry |
| `J` | Select current entry and move down (range select) |
| `K` | Select current entry and move up (range select) |

Selected entries are highlighted in the file listing and used by bulk
operations such as `C` and `X`.

---

## Preview Modes

These keys toggle different views in the right preview pane. Toggle the same
key again to return to the default file content preview.

| Key | Action |
|-----|--------|
| `d` | Toggle git diff preview |
| `m` | Toggle metadata card |
| `V` | Toggle git log preview |
| `a` | Toggle hex dump preview |
| `D` | Toggle disk usage preview (directories only) |
| `f` | Compare two selected files (select exactly 2 first) |
| `#` | Toggle line numbers in preview pane |
| `U` | Toggle word wrap in preview pane |

---

## Archives

| Key | Action |
|-----|--------|
| `Z` | Extract archive into current directory |

Supported formats depend on the archive tools available on the system.

---

## View and Display

| Key | Action |
|-----|--------|
| `.` | Toggle hidden files |
| `S` | Cycle sort mode (name → size → modified → type) |
| `s` | Toggle sort order (ascending ↔ descending) |
| `T` | Toggle timestamps (replaces size column) |
| `N` | Toggle directory item counts |
| `i` | Toggle gitignore filter |
| `w` | Collapse / expand preview pane |
| `I` | Toggle watch mode off/on (on by default; disables auto-refresh when toggled off) |

---

## Yank and Clipboard

These bindings copy a path to the system clipboard using the OSC 52 escape
sequence, which works over SSH and inside tmux / cmux.

| Key | Action |
|-----|--------|
| `y` | Yank relative path |
| `Y` | Yank absolute path |
| `A` | Yank path with format picker (relative / absolute / filename / parent) |

---

## Bookmarks and Marks

Trek has two bookmark mechanisms:

- **Bookmarks** (`b` / `B`): always persist across sessions, stored on disk.
- **Session marks** (`` ` `` / `'`): saved on clean quit (`q` / `Q`) and restored on next launch without arguments.

| Key | Action |
|-----|--------|
| `b` + letter | Save bookmark at current directory (persists across sessions) |
| `B` + letter | Jump to saved bookmark |
| `` ` `` + letter | Set session mark at current directory |
| `'` + letter | Jump to session mark |

Any letter `a`–`z` is a valid mark or bookmark slot.

---

## Search

| Key | Action |
|-----|--------|
| `/` | Start fuzzy file name search |
| `Ctrl+F` | Start ripgrep content search |
| `Tab` / `↓` | Next search match |
| `Shift+Tab` / `↑` | Previous search match |
| `Enter` | Confirm search selection |
| `Esc` | Cancel search |

Fuzzy search matches against file names in the current directory tree.
Content search (`Ctrl+F`) uses ripgrep and respects `.gitignore` by default.

---

## App

| Key | Action |
|-----|--------|
| `:` | Open command palette |
| `?` | Show help overlay |
| `q` | Quit |

---

## Mouse

| Action | Effect |
|--------|--------|
| Click | Select entry / enter directory |
| Right-click | Select entry and open it in a new cmux tab (same routing as `l` / `Enter`) |
| Double-click | Open file in a new cmux pane split to the right; falls back to system opener for images, HTML, and PDFs |
| Drag pane dividers | Resize panes |
| Scroll wheel over preview | Scroll preview content |

Mouse support is enabled by default. All mouse actions have keyboard
equivalents. Right-click and double-click both appear in the help overlay (`?`)
and the command palette.

---

## Session Restore

When you quit Trek with `q` or `Q`, Trek saves your view state and restores it
the next time you launch without arguments. The state that persists includes:
current directory, selected entry, marks, hidden-files toggle (`.`), sort mode
(`S`), and sort order (`s`).

Launching with an explicit path (`trek /path`) always ignores saved session
state. See [Navigation — Session Restore](../usage/navigation.md#session-restore)
for full details.

---

## See Also

- [Command Palette](command-palette.md) — run any action by typing its name
- [Navigation guide](../usage/navigation.md) — detailed navigation walkthrough
