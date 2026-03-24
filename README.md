# trek

A terminal file manager with mouse-resizable panes, inspired by yazi and ranger.

![trek screenshot](https://raw.githubusercontent.com/bradleyfay/trek/main/assets/demo.gif)

## Install

```sh
brew install bradleyfay/trek/trek
```

### Shell integration

The `m` function lets you navigate with trek and `cd` into the selected directory when you quit:

```sh
trek --install-shell
```

Then reload your shell (`source ~/.zshrc` or open a new terminal) and use `m` to launch trek.

## Features

- **Three-pane layout**: parent directory, current directory, and file preview
- **Mouse-resizable panes**: drag the dividers between panes to resize them
- **Scroll wheel preview**: scroll through file previews with your mouse wheel
- **File preview**: text files are previewed in the right pane; directories show their contents
- **Nerd font icons**: file-type icons for 100+ extensions and special directories
- **Fuzzy search**: press `/` to incrementally filter files with fuzzy matching
- **Yank to clipboard**: `y` copies relative path, `Y` copies absolute path via OSC 52
- **Keyboard navigation**: vim-style keybindings (h/j/k/l)

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Go to parent directory |
| `l` / `→` / `Enter` | Enter directory |
| `g` | Go to top |
| `G` | Go to bottom |
| `~` | Go to home directory |
| `.` | Toggle hidden files |
| `/` | Start fuzzy search |
| `y` | Yank relative path to clipboard |
| `Y` | Yank absolute path to clipboard |
| `?` | Show help overlay |
| `q` | Quit |

### Search mode

| Key | Action |
|-----|--------|
| Type | Filter files (fuzzy match) |
| `Tab` / `↓` | Next match |
| `Shift+Tab` / `↑` | Previous match |
| `Enter` | Confirm selection |
| `Esc` | Cancel |

### Mouse

- **Drag dividers** between panes to resize them
- **Scroll wheel** over the preview pane to scroll file contents
- **Click** to select a file or enter a directory

## Build from source

```sh
cargo build --release
```

## License

MIT
