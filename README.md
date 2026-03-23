# Murdoch

A terminal file manager with mouse-resizable panes, inspired by yazi and ranger.

## Features

- **Three-pane layout**: parent directory, current directory, and file preview
- **Mouse-resizable panes**: drag the dividers between panes to resize them
- **Scroll wheel preview**: scroll through file previews with your mouse wheel
- **File preview**: text files are previewed in the right pane; directories show their contents
- **Nerd font icons**: file-type icons for 100+ extensions and special directories
- **Fuzzy search**: press `/` to incrementally filter files with fuzzy matching
- **Yank to clipboard**: `y` copies relative path, `Y` copies absolute path via OSC 52
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
| `/` | Start fuzzy search |
| `y` | Yank relative path to clipboard |
| `Y` | Yank absolute path to clipboard |
| `q` | Quit |

### Search Mode

| Key | Action |
|-----|--------|
| Type characters | Filter files (fuzzy match) |
| `Tab` / `Down` | Next match |
| `Shift+Tab` / `Up` | Previous match |
| `Enter` | Confirm selection |
| `Esc` | Cancel search |

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
