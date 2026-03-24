# trek

When you work with an AI coding assistant, the project directory becomes a black box. Files appear, change, and move while you're focused on the conversation. You lose track of what actually exists, what got modified, and whether the structure makes sense.

Trek is a terminal file browser that runs in a persistent pane alongside your AI session. You can see the file tree update as the AI works, preview what it wrote, check git status without switching context, and navigate to anything without typing full paths.

It runs inside [cmux](https://github.com/bradleyfay/cmux). It is not a text editor.

![trek screenshot](https://raw.githubusercontent.com/bradleyfay/trek/main/assets/demo.gif)

## Install

```sh
brew install bradleyfay/trek/trek
```

### Shell integration

```sh
trek --install-shell
```

Adds an `m` function to your shell that launches trek and `cd`s into whatever directory you were in when you quit.

## What it does

- Three-pane layout: parent directory, current directory, file preview
- File tree auto-refreshes when the filesystem changes (watch mode on by default)
- Git status shown inline — modified, staged, untracked, deleted
- Full-text search across the project via ripgrep (`Ctrl+F`)
- Fuzzy file name filtering (`/`)
- Opens files in the right tool: code in `$EDITOR`, HTML/images/PDFs via system default
- Yank file paths to clipboard via OSC 52 (`y` relative, `Y` absolute)
- Mouse-resizable panes; mouse and keyboard both work throughout

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Parent directory |
| `l` / `→` / `Enter` | Enter directory / open file |
| `g` / `G` | Top / bottom |
| `~` | Home directory |
| `.` | Toggle hidden files |
| `/` | Fuzzy file search |
| `Ctrl+F` | Full-text search (ripgrep) |
| `y` / `Y` | Yank relative / absolute path |
| `?` | Help overlay |
| `:` | Command palette |
| `q` | Quit |

Press `:` or `?` to see everything else.

## Build from source

```sh
cargo build --release
```

## License

MIT
