# File Operations

Trek handles the full range of day-to-day file management tasks: creating, copying, moving, renaming, deleting, and organizing files. Most operations apply to the currently selected entry or to the current selection set.

---

## Opening Files

| Input | Action |
|-------|--------|
| `l` / `→` / `Enter` | Enter a directory; for files, open in a new cmux tab (routes by file type — see below) |
| Right-click | Select the file and open it in a new cmux tab (same routing as `l` / `Enter`) |
| Double-click | Open the file in a new cmux pane split to the right (`cmux new-pane --direction right`); falls back to system opener for images, HTML, and PDFs |
| `o` | Open in terminal editor — checks `$VISUAL`, then `$EDITOR`, then falls back to `vi` |
| `O` | Open with system default — `open` on macOS, `xdg-open` on Linux |

### File routing

When you press `l`, `→`, or `Enter` on a file, right-click a file, or double-click a file, Trek routes it based on type:

| File type | `l` / `Enter` / right-click opens with | Double-click opens with |
|-----------|----------------------------------------|------------------------|
| HTML (`.html`, `.htm`) | System default opener (`open` / `xdg-open`) | System default opener |
| Images (`.png`, `.jpg`, `.gif`, etc.) | System default opener | System default opener |
| PDFs (`.pdf`) | System default opener | System default opener |
| All other text / code files | `$EDITOR` in a new cmux tab | `$EDITOR` in a new cmux pane split right |

This requires Trek to be running inside cmux. When Trek is not running inside cmux, all three open methods show a hint in the status bar instead. Use `o` or `O` as alternatives in that case.

To copy a file's path to the clipboard instead of opening it, use `y` (relative path) or `Y` (absolute path) from the Yanking Paths section below.

---

## Creating Files and Directories

| Key | Action |
|-----|--------|
| `M` | Create a new directory — opens an input bar; press `Enter` to confirm |
| `t` | Touch / create a new empty file |
| `W` | Duplicate the current entry in place — pre-fills the input bar with a suggested name (e.g. `file_copy.txt`) |
| `L` | Create a symlink to the current entry — pre-fills the entry name; the symlink is created at `cwd/<name>` |

---

## Copying and Moving

Trek uses a clipboard model: copy or cut entries first, then paste them into the target directory after navigating there.

| Key | Action |
|-----|--------|
| `c` | Copy the current entry to the clipboard |
| `C` | Copy all selected entries to the clipboard (displays total size) |
| `x` | Cut the current entry |
| `X` | Cut all selected entries |
| `p` | Paste clipboard contents into the current directory |
| `F` | Open the clipboard inspector — shows queued items color-coded by operation (green = copy, yellow = cut); press `p` inside to paste, `Esc` to close |

---

## Deleting

| Key | Action |
|-----|--------|
| `d` | Delete or trash the current entry — requires confirmation |

Bulk deletion uses `X` (cut all selected entries) combined with a delete confirmation, or select entries first and then use `d`.

---

## Renaming

| Key | Action |
|-----|--------|
| `n` / `F2` | Quick rename — opens an inline input bar pre-filled with the current name |

---

## Selection

Build a selection set before running bulk operations like copy or cut.

| Key | Action |
|-----|--------|
| `Space` | Toggle selection on the current entry |
| `J` (Shift+J) | Select current entry and move cursor down (range select) |
| `K` (Shift+K) | Select current entry and move cursor up (range select) |

---

## Yanking Paths

Copy file paths to the system clipboard using OSC 52 (works in most modern terminals):

| Key | Action |
|-----|--------|
| `y` | Yank the relative path |
| `Y` | Yank the absolute path |
| `A` | Open the path format picker — choose from `r` (relative), `a` (absolute), `f` (filename only), or `p` (parent directory) |

---

## Bookmarks

Bookmarks save directories to disk and persist across Trek sessions. They are stored in `~/.local/share/trek/`.

| Key | Action |
|-----|--------|
| `b` + letter | Save the current directory as a bookmark at that letter slot |
| `B` + letter | Jump to the saved bookmark at that letter slot |

For temporary, session-only location pinning, use marks instead. See [Navigation — Marks vs. Bookmarks](navigation.md#marks-vs-bookmarks) for the distinction.
