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
- Browse archive contents without extracting — press `l`/`Enter` on any `.zip`, `.jar`, `.tar.gz`, `.tgz`, or similar archive to enter a virtual filesystem browser with the same three-pane layout; navigate inside it exactly like the real filesystem and press `Esc` to return
- Preview pane renders on a background thread — Trek stays interactive while large files highlight or diffs compute; navigating away cancels any in-flight render so stale results are never shown
- Images (`.png`, `.jpg`, `.gif`, `.webp`, and others) show a metadata card with format, file size, and pixel dimensions; when `chafa` is installed, they also render as inline Unicode/sixel art at 72 columns
- PDFs show format, PDF version, and file size; when `pdfinfo` (poppler-utils) is installed, full document metadata is shown instead
- File tree auto-refreshes when the filesystem changes (watch mode on by default)
- Live change feed shows real-time filesystem events as they happen (`F`)
- Copy, move, and archive extraction run on background threads — Trek stays interactive during large transfers; monitor progress in the task manager (`Ctrl+T`)
- Git status shown inline — modified, staged, untracked, deleted
- Full-text search across the project via ripgrep (`Ctrl+F`)
- Fuzzy file name filtering (`/`)
- Opens files in the right tool — configurable via `~/.config/trek/opener.conf`, with sensible defaults
- Yank file paths to clipboard via OSC 52 (`y` relative, `Y` absolute)
- Mouse-resizable panes; mouse and keyboard both work throughout

## Opening files

When you open a file, Trek looks up the first matching rule in `~/.config/trek/opener.conf` (or `$XDG_CONFIG_HOME/trek/opener.conf`) and runs the specified command. Rules are evaluated top-to-bottom; the first match wins.

### Config format

```
# This is a comment
ext <ext1|ext2|...> : <command>
glob <pattern>      : <command>
```

- `ext` matches by file extension (case-insensitive, no leading dot).
- `glob` matches the full filename against a shell glob pattern.
- `{}` is replaced with the file path when the command is run.
- Commands run via `sh -c`.

### Example

```
# Open markdown in the cmux viewer
ext md|markdown : cmux open --md {}

# Open HTML in the system browser
ext html|htm : open {}

# Open images in Preview
ext png|jpg|jpeg|gif|webp : open {}

# Fall back to VS Code for everything else
glob * : code {}
```

### Built-in defaults

If no config file exists, Trek falls back to:

| File type | Default action |
|-----------|----------------|
| Markdown (`.md`) | cmux markdown viewer |
| HTML / images / PDFs | System default (`open` / `xdg-open`) |
| Code / text | `$EDITOR` in a new cmux surface |
| Directories | Navigate in-place |

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
| `F` | Toggle live change feed |
| `Ctrl+T` | Task manager (background copy/move/extract operations) |
| `F9` | Clipboard inspector |
| `I` | Toggle watch mode (pauses change feed when off) |
| `?` | Help overlay |
| `:` | Command palette |
| `Esc` | Exit archive mode / dismiss overlay |
| `q` | Quit |

Press `:` or `?` to see everything else.

## Archive navigation

Pressing `l` or `Enter` on any supported archive enters a virtual filesystem browser. The three-pane layout, preview pane, and all navigation keys work exactly as they do on the real filesystem.

**Supported formats**

| Format | Extensions | Implementation |
|--------|------------|----------------|
| Zip-family | `.zip`, `.jar`, `.war`, `.ear` | Bundled `zip` crate — no external tools needed |
| Tar (uncompressed) | `.tar` | System `tar` |
| Tar + Gzip | `.tar.gz`, `.tgz` | System `tar` |
| Tar + Bzip2 | `.tar.bz2` | System `tar` |
| Tar + XZ | `.tar.xz` | System `tar` |
| Tar + Zstandard | `.tar.zst` | System `tar` |

**Navigating inside an archive**

- `j` / `k` — move down / up through entries
- `l` / `Enter` — step into a virtual directory, or extract a file to a temp directory and show its preview
- `h` / `←` — step back out to the parent virtual directory
- `Esc` — exit archive mode entirely and return to the real filesystem

The path bar shows a breadcrumb for your position inside the archive, for example `archive.zip / src / utils`, with navigation hints while archive mode is active.

## Build from source

```sh
cargo build --release
```

### Optional tools

These tools are not required but enhance the preview pane when present:

| Tool | What it enables |
|------|-----------------|
| `chafa` | Renders raster images as inline Unicode/sixel art in the preview pane |
| `pdfinfo` (poppler-utils) | Shows full document metadata for PDF files |

Trek detects both at runtime. If either is absent, it falls back gracefully and shows a short install hint in the preview pane.

## License

MIT
