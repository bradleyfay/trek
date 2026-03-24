# Quick Start

This page gets you productive with Trek in a few minutes. You do not need to memorize keybindings — Trek is designed to be discoverable from the start.

---

## Launch Trek

If you have set up shell integration:

```sh
m
```

Or run Trek directly without shell integration:

```sh
trek
```

Trek opens in the current directory. If you have previously quit Trek cleanly (with `q`), it restores your last session: the directory you were in, your cursor position, and your view settings (hidden files, sort order). Pass an explicit path to skip session restore and open at a specific location instead:

```sh
trek /path/to/project
```

---

## The Three-Pane Layout

Trek divides the screen into three panels:

| Pane | What it shows |
|---|---|
| Left | The parent directory of your current location |
| Center | The current directory — this is where you navigate |
| Right | A preview of the selected file or the contents of a selected directory |

The center pane is where your cursor lives. The left and right panes update automatically as you move.

You can resize any pane by clicking and dragging the dividers between them.

---

## Basic Navigation

Trek supports both keyboard and mouse. Use whichever feels natural.

**Keyboard:**

| Key | Action |
|---|---|
| `j` or `Down` | Move cursor down |
| `k` or `Up` | Move cursor up |
| `h` or `Left` | Go to parent directory |
| `l`, `Enter`, or `Right` | Enter directory / open file |
| `g` | Jump to top of list |
| `G` | Jump to bottom of list |

**Mouse:**

- Click any entry to select it
- Double-click a directory to enter it
- Scroll the mouse wheel to move through the list
- Scroll inside the preview pane to read longer files
- Drag the dividers between panes to resize them

---

## Opening Files

When a file is selected in the center pane, you have two ways to open it:

| Key | Action |
|---|---|
| `o` | Open in `$EDITOR` (your default terminal editor) |
| `O` | Open with the system default application |

Trek routes files based on type. Directories are always navigated in-place. Other file types open in the appropriate external tool.

---

## Getting Help

You do not need to memorize every keybinding. Two overlays surface everything you need:

**Help overlay** — press `?` to see a summary of all keybindings available in the current context. Press `?` again or `Escape` to close it.

**Command palette** — press `:` to open a searchable list of every available action. Type part of an action name to filter the list, then press `Enter` to run it. The palette also shows the keybinding for each action so you can learn as you go.

---

## Quitting

Press `q` to quit Trek. Quitting cleanly saves your session state — current directory, cursor position, marks, hidden-files toggle, and sort settings — so Trek can restore it on the next launch.

If you launched Trek with the `m` shell function, your terminal session will also `cd` to the directory Trek had open when you quit.
