# Keybinding Reference

Trek v0.48.1

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
| `l` / `→` / `Enter` | Enter directory or open file |
| `g` | Jump to top of listing |
| `G` | Jump to bottom of listing |
| `~` | Go to home directory |
| `e` | Open path jump bar (type any path) |
| `` ` `` + letter | Set session mark at current directory |
| `'` + letter | Jump to session mark |
| `z` | Open frecency jump list |
| `Ctrl+O` | Navigate back in history |
| `Ctrl+I` | Navigate forward in history |
| `[` | Scroll preview pane up 5 lines |
| `]` | Scroll preview pane down 5 lines |

Session marks are in-memory only and do not persist across Trek sessions. For
persistent bookmarks, see the Bookmarks section below.

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
| `r` | Bulk rename selected files (regex) |

---

## Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle selection on current entry |
| `J` | Select current entry and move down (range select) |
| `K` | Select current entry and move up (range select) |
| `*` | Select files by glob pattern |

Selected entries are highlighted in the file listing and used by bulk
operations such as `C`, `X`, `r`, and `E`.

---

## Preview Modes

These keys toggle different views in the right preview pane. Toggle the same
key again to return to the default file content preview.

| Key | Action |
|-----|--------|
| `d` | Toggle git diff preview |
| `m` | Toggle metadata card |
| `H` | Toggle SHA-256 hash preview |
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
| `E` | Create archive from current entry or selection |

Supported formats for extraction depend on the archive tools available on the
system. Creation always produces a `.tar.gz` archive.

---

## View and Display

| Key | Action |
|-----|--------|
| `.` | Toggle hidden files |
| `T` | Toggle timestamps (replaces size column) |
| `N` | Toggle directory item counts |
| `i` | Toggle gitignore filter |
| `w` | Collapse / expand preview pane |
| `I` | Toggle watch mode (auto-refresh on filesystem changes) |

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

- **Bookmarks** (`b` / `B`): persist across sessions, stored on disk.
- **Session marks** (`` ` `` / `'`): in-memory only, lost on quit.

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
| Drag pane dividers | Resize panes |
| Scroll wheel over preview | Scroll preview content |

Mouse support is enabled by default. All mouse actions have keyboard
equivalents.

---

## See Also

- [Command Palette](command-palette.md) — run any action by typing its name
- [Navigation guide](../usage/navigation.md) — detailed navigation walkthrough
