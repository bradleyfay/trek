# Murdoch

A terminal file manager with mouse-resizable panes, inspired by yazi and ranger.

## Features

- **Three-pane layout**: parent directory, current directory, and file preview
- **Mouse-resizable panes**: drag the dividers between panes to resize them
- **Scroll wheel preview**: scroll through file previews with your mouse wheel
- **File preview**: text files are previewed in the right pane; directories show their contents
- **Keyboard navigation**: vim-style keybindings

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `h` / `Left` | Go to parent directory |
| `l` / `Right` / `Enter` | Enter directory |
| `g` | Go to top |
| `G` | Go to bottom |
| `~` | Go to home directory |
| `q` | Quit |

## Mouse

- **Drag dividers** between panes to resize them
- **Scroll wheel** over the preview pane to scroll file contents

## Building

```
cargo build --release
```

## Running

```
cargo run
```
