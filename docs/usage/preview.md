# Preview Pane

The right pane in Trek continuously previews the selected entry. By default it shows file contents with syntax highlighting. A set of preview modes let you switch the pane to show different information about the same file without navigating away.

---

## Default Preview

- **Text files** — displayed with syntax highlighting
- **Directories** — show their contents as a listing
- **Raster images** (PNG, JPEG, GIF, BMP, ICO, WEBP, AVIF, TIFF) — show a metadata card with format, file size, and pixel dimensions. When `chafa` is installed, Trek also renders an inline image using Unicode block characters or sixels directly in the preview pane.
- **SVG** — previewed as syntax-highlighted XML, the same as any text file
- **Binary files** — show a placeholder message

---

## Preview Modes

Preview modes are toggled by key and are mutually exclusive with each other. Pressing the same key again returns the pane to the default text preview.

| Key | Mode | Description |
|-----|------|-------------|
| `d` | Diff preview | Shows `git diff HEAD -- <file>` for the selected file |
| `m` | Meta card | Shows file metadata: permissions, size, and timestamps. For text files also shows line, word, and character counts. For symlinks shows the target path and whether it resolves. |
| `H` | Hash preview | Shows the SHA-256 checksum using `shasum -a 256` or `sha256sum`. Files larger than 512 MB show a size-limit message instead. |
| `V` | Git log preview | Shows `git log --oneline -30 -- <path>`. Works for directories as well as files — directories show commits that touched any file in the subtree. |
| `a` | Hex dump | Shows a hex dump via `xxd`, falling back to `hexdump -C` if `xxd` is not available. Files larger than 4 MB show a size-limit message. |
| `D` | Disk usage | For directories: shows immediate children sorted largest-first with Unicode block bars representing relative size. Pressing `D` on a file shows an error. |
| `f` | Compare two files | Select exactly two files with `Space`, then press `f` to display a unified diff (`diff -u`) between them. |

---

## Display Options

These toggles apply on top of whichever preview mode is active:

| Key | Option |
|-----|--------|
| `#` | Toggle line numbers — adds a dark gray gutter with absolute line numbers |
| `U` | Toggle word wrap — soft-wraps long lines at the pane boundary; shows a `[wrap]` indicator in the title |

---

## Scrolling the Preview

| Key | Action |
|-----|--------|
| `[` | Scroll up 5 lines |
| `]` | Scroll down 5 lines |
| Mouse scroll wheel | Scroll 3 lines per event |

Scrolling works in all preview modes.

---

## Preview Pane Title Indicators

The preview pane title bar shows a badge identifying the active mode or display option:

| Badge | Meaning |
|-------|---------|
| `[diff]` | Git diff mode active |
| `[meta]` | Metadata view active |
| `[hash]` | Hash view active |
| `[log]` | Git log mode active |
| `[hex]` | Hex dump mode active |
| `[du]` | Disk usage mode active |
| `[compare]` | Two-file compare active |
| `[wrap]` | Word wrap enabled |

Multiple badges can appear at once when a mode and a display option are both active — for example `[log]` and `[wrap]` together.

---

## Optional Dependencies

Some preview features require external tools. Trek detects them at runtime and falls back gracefully when they are absent, showing a short install hint in the preview pane.

| Tool | Purpose | Install (macOS) |
|------|---------|-----------------|
| `chafa` | Inline image rendering — renders raster images as Unicode block characters or sixels in the preview pane | `brew install chafa` |
| `pdfinfo` (poppler-utils) | Full document metadata for PDF files | `brew install poppler` |
