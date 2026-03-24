# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.15.0] - 2026-03-24

### Added
- **`m` — metadata preview**: toggles the preview pane to a structured info card showing path, type, size (human-readable + raw bytes), Unix mode (symbolic `rwxrwxrwx` and octal), UID/GID, modified time, and accessed time; pane title shows `filename [meta]`; mutually exclusive with diff mode (`d`)
- **`P` — chmod editor**: opens an inline input bar showing the current octal mode; input restricted to digits 0–7 (max 4 chars); `Enter` applies via `std::fs::set_permissions`, `Esc` cancels; metadata card refreshes immediately on success; non-Unix platforms show a descriptive message
- `m` and `P` documented in the help overlay (`?`) under View
- New helper functions: `format_permission_bits`, `meta_human_size`, `format_unix_timestamp_utc` (pure Rust UTC arithmetic, no subprocess)
- 6 new unit tests covering permission bit formatting, human-readable sizes, and UTC timestamp formatting (epoch + known date)

## [0.14.0] - 2026-03-24


### Changed
- `Delete` and `X` now move files to the platform trash (recoverable) instead of permanent deletion; the confirmation prompt now reads `[t/y]trash  [D]delete permanently  [Esc]cancel`
- Status message after trash shows `Trashed N items [u to undo]` to make the undo path discoverable

### Added
- **Trash / soft-delete**: files moved to `~/.Trash` (macOS) or `$XDG_DATA_HOME/Trash/files` (Linux/other) instead of being permanently removed
- **Linux `.trashinfo` sidecars**: written to `$XDG_DATA_HOME/Trash/info/` for Nautilus/Thunar compatibility; deletion timestamp formatted as ISO 8601 UTC using pure Rust arithmetic
- **Collision handling**: if a same-named file already exists in the trash, appends ` (2)`, ` (3)`, … to the stem (up to 100 attempts)
- `u` undoes the last trash operation, restoring all items in that group to their original paths; shows `Restored: name` or an error if the trash slot is gone
- `[D]` in the confirmation prompt permanently deletes immediately (previous `y` behaviour, now explicit)
- New `src/trash.rs` module: `trash_path`, `restore_path`, `platform_trash_dir`, `unique_trash_dest`; 6 unit tests covering no-collision dest, one-collision dest, dotfile naming, trash→restore roundtrip, missing-file error, and platform dir resolution

## [0.13.0] - 2026-03-24


### Added
- Persistent directory bookmarks: `b` saves the current directory; `B` opens a centered picker overlay
- Bookmarks stored at `$XDG_DATA_HOME/trek/bookmarks` (fallback: `~/.local/share/trek/bookmarks`) — plain text, one path per line, insertion order
- Duplicate paths silently deduplicated on `b`
- Picker supports `j`/`k` and arrow navigation; `Enter` jumps to the selected bookmark and pushes a history entry; `Esc` or `B` closes without navigating; `d` removes the focused bookmark instantly
- Typing while the picker is open filters by name or path; `Backspace` removes the last filter character
- Stale bookmarks (non-existent paths) shown dimmed with `[gone]`; navigating to one shows an error message instead of crashing
- Empty state shows `"No bookmarks — press b to add one"` in the picker
- Help overlay documents `b` and `B` under the Search section
- New `src/bookmarks.rs` module: `load`, `add`, `remove`, `save`; 6 unit tests covering load-empty, add-then-load, deduplication, remove-at-index, out-of-range remove, and XDG path resolution

## [0.12.0] - 2026-03-24

### Added
- Recursive filename find (`Ctrl+P`): live search across all files under the current directory, updating results on every keystroke
- New `src/find.rs` module: `run_find()` prefers `fd` when available and falls back to a built-in directory walker; walker skips hidden dirs, `target/`, and `node_modules/`
- Results capped at 500 entries with a `[truncated]` notice in the pane title
- Results sorted by relevance: exact filename/stem match → prefix → substring
- `j`/`k` navigate results; `l`/`Enter`/`→` jump to the selected file (navigates to its parent directory, selects the file, exits find mode, and pushes a history entry); `Esc` or `Ctrl+P` again cancels without side effects
- `Ctrl+P` added to the help overlay (`?`) under the Search section
- 7 unit tests covering output parsing, empty-query short-circuit, truncation, relevance sorting, walker finds, and hidden-directory skip

## [0.11.0] - 2026-03-23

### Changed
- **Consistent pane borders**: parent pane now uses `TOP | RIGHT`, current pane uses `TOP | RIGHT`, preview pane uses `TOP` only — eliminates the double-border artifact between panes
- **Unified selection highlight**: `bg: Blue, fg: White` used everywhere (current pane cursor, content search selected), replacing the inconsistent `LightYellow`; directory entries now use `Cyan` (not `Blue`) to avoid colliding with the selection background
- **Active pane emphasis**: current pane border and title rendered in `Cyan`/`White+Bold`; parent and preview pane borders/titles remain `DarkGray` — gives immediate spatial orientation
- **Styled pane titles**: current pane title `White+Bold` (active); parent and preview titles `DarkGray` (context)
- **Dimmed file sizes**: size column always rendered in `DarkGray` (or `Gray` on cursor row) to visually separate it from filenames
- **Long filename truncation**: names wider than available column space are truncated with `…` instead of a hard cut; applied in both parent and current panes
- **Symbolic git status glyphs**: `M`→`●`, staged→`✚`, conflict/deleted→`✖`, untracked→`+`, dirty dir→`●` — more universally recognizable than bare letters
- **Smart path bar truncation**: when path is wider than the terminal, shows last 3 components with `…/` prefix; hidden-files `[H]` rendered as a separate dimmed span
- **Improved scrollbar**: track uses `Color::Rgb(60,60,60)` (visible on dark terminals), thumb uses `Color::Gray`; track is always shown when content overflows
- **Content search**: file group headers use `Cyan` (was `Blue`)
- **Permanent selection prefix**: 2-char selection prefix space is always reserved, eliminating layout shift when entering/leaving selection mode
- **Help overlay**: keybindings grouped into labeled sections (Navigation, Search, View, Selection & Rename, File Operations, Yank & Misc); overlay widened to 60 columns; "Any key to close" hint added at the bottom; border color changed to `Cyan` for consistency
- **Path bar color**: path text now uses `White+Bold` for better readability
- 5 unit tests for `truncate_with_ellipsis` covering normal, at-limit, truncation, min-width, and Unicode cases

## [0.10.0] - 2026-03-23

### Added
- Directory jump history: `Ctrl+O` goes back to the previous visited directory; `Ctrl+I` goes forward after going back
- Each history entry stores the directory path and cursor index; cursor is restored on return (clamped if entries changed)
- Forward entries are discarded when navigating somewhere new (browser-style)
- Stack capped at 50 entries; oldest entries dropped when full
- Status message shows direction and stack position: `← 3/7  ~/projects/myapp/src`
- History is recorded on `l`/`Enter`, `h`, `~`, mouse parent-pane click, and content search jump
- `Ctrl+O`/`Ctrl+I` themselves do not push new entries
- Navigating to a deleted history entry shows a descriptive error message instead of crashing
- Help overlay documents both new keybindings
- 5 unit tests covering initialization, boundary messages, forward-discard semantics, and stack cap

## [0.9.0] - 2026-03-23

### Added
- Configurable directory sort modes: `S` cycles through Name → Size → Modified → Extension → Name; `s` toggles ascending/descending
- Size and Modified default to descending (largest/newest first); Name and Extension default to ascending
- Sort indicator shown in the path bar when not using the default sort (`↓ Size`, `↑ Modified`, etc.)
- Cursor follows the selected file by name after a sort change — no jump to top
- Sort mode is applied on every directory navigation and persists for the session
- `DirEntry` gains a `modified: SystemTime` field populated from filesystem metadata
- Help overlay documents `S` and `s`
- 7 new unit tests covering sort mode cycling, labels, sort-by-name/size/modified/extension, and the invariant that directories always precede files

## [0.8.0] - 2026-03-23

### Added
- Archive content preview: hovering over `.zip`, `.jar`, `.war`, `.ear`, `.tar`, `.tar.gz`/`.tgz`, `.tar.bz2`/`.tbz2`, `.tar.xz`/`.txz`, `.tar.zst`/`.tzst`, `.gz`, and `.7z` files shows the archive's file manifest in the preview pane instead of `[binary file]`
- No extraction occurs — only the table of contents is listed
- When a required tool (`unzip`, `7z`) is not installed, an informative message is shown instead of crashing
- Archive listings capped at 1,000 entries with a `[truncated]` notice appended
- Corrupt or unreadable archives show `[could not read archive]`
- New `src/archive.rs` module: `try_list_archive()`, per-format parsers, and 16 unit tests covering extension detection, output parsing, and truncation

## [0.7.0] - 2026-03-23

### Added
- Syntax-highlighted file preview: source files (`.rs`, `.py`, `.js`, `.ts`, `.yaml`, `.json`, and 40+ other extensions) are now rendered with per-token color in the preview pane using `syntect` (same engine as `bat`/`delta`)
- New `src/highlight.rs` module: `Highlighter` struct loaded once at startup; `highlight()` returns `None` for unrecognized extensions so the caller falls back to plain text automatically
- Theme: `base16-ocean.dark` applied to all highlighted previews
- Syntax highlighting capped at `preview_scroll + visible_height` lines for bounded render time
- Diff preview (`d`) and plain-text fallback unaffected by the new code path

## [0.6.1] - 2026-03-24

### Added
- `trek <path>` now opens in the specified directory instead of always using CWD; supports absolute, relative, and `~`-expanded paths; validates that the path is a directory and exits 1 with an error message if not (#8)
- `trek --version` / `trek -V` prints `trek <version>` and exits 0 instead of launching the TUI (#6)
- `trek --help` / `-h` now documents the optional `[PATH]` positional argument and the `-V`/`--version` flag; `--choosedir` removed from user-facing help (internal shell-integration flag) (#10)

### Fixed
- Unrecognized flags (e.g. `trek --typo`) now print an error to stderr and exit 1 instead of silently launching the TUI (#7)
- Trek now exits with code 1 when `run()` returns an error, instead of always exiting 0 (#9)

### Changed
- Argument parsing extracted into a testable `parse_args` function with 9 unit tests covering all flag combinations and error cases

## [0.6.0] - 2026-03-24

### Added
- File copy: `c` copies the current file/dir to the clipboard; `C` copies all selected files
- File cut: `x` marks the current file/dir for move (cut)
- Paste: `p` pastes clipboard contents into the current directory; conflicting names are skipped with a status message; cut operations use atomic `rename` (same-filesystem) with copy+delete fallback (cross-device)
- Recursive directory copy and move supported
- Delete: `Delete` key deletes the current file/directory with a confirmation prompt (`y` confirms, any other key cancels); `X` deletes all selected files with confirmation
- Make directory: `M` opens an inline input bar; `Enter` creates the directory, `Esc` cancels
- Clipboard indicator shown in status bar: `[copy] N files` or `[cut] N files` while clipboard is populated, with paste hint
- Directory listing and git status are refreshed after every mutating operation
- New `src/ops.rs` module: `copy_path`, `move_path`, `delete_path`, `make_dir`; eight unit tests covering same-dir copy, cross-dir copy, recursive dir copy, same-fs move, file delete, dir delete, mkdir success, mkdir-already-exists error

## [0.5.0] - 2026-03-24

### Added
- Content search mode (`Ctrl+F`): spawns `rg --line-number --color never` in the current directory and displays grouped results in the center pane
- Results grouped by file with line numbers; j/k navigate across all matches; `l`/`→` or `Enter` jumps to the matched file and scrolls the preview to the matching line
- Clear error message shown in status bar when `rg` is not installed (`content search requires ripgrep (rg) — not found in PATH`)
- Results capped at 500 matches with a visible `[truncated]` notice in the pane title
- `Esc` returns to normal navigation without side effects
- New `src/search.rs` module with `run_rg` and `parse_rg_output`; six unit tests covering grouping, empty output, colons in content, result capping, and malformed lines
- `Ctrl+F` added to the help overlay (`?`) and `--help` output

## [0.4.0] - 2026-03-24

### Added
- Bulk rename mode: select files with `Space`, `v` to select all, `r` to open rename bar
- Live rename preview in center pane showing `old → new`, `[no match]`, and `[conflict]` rows
- Regex pattern matching with full capture group support (`$1`, `$2`, etc.)
- Template tokens in replacement field: `{n}`, `{n:02}`, `{stem}`, `{ext}`, `{date}`
- Two-pass conflict detection against the post-rename namespace (avoids ordering bugs)
- `Esc` clears selections in normal mode; `Esc` in rename mode cancels without touching filesystem
- Selection count and hint shown in status bar while files are marked

### Fixed
- `cargo fmt` violations in `draw_rename_bar` (lines too long)
- `clippy::ptr_arg` in `load_git_diff` (changed `&PathBuf` to `&Path`)
- `clippy::manual_flatten` in rename preview batch-count loop
- Deprecated `macos-13` runner replaced with `macos-14` cross-compilation for `x86_64-apple-darwin`

## [0.3.0] - 2026-03-24

### Added
- Git status integration: inline `M` (modified), `S` (staged), `!` (conflict), `D` (deleted), `?` (untracked) indicators on files in the current pane
- `~` indicator on directories that contain any changed files
- Branch name displayed in path bar: `~/project/src  (main)` or `(HEAD:abc1234)` for detached HEAD
- `d` key toggles diff preview — preview pane shows `git diff` / `git diff --cached` output with colorized `+`/`-`/`@@` lines
- `R` key manually refreshes git status
- Git status cached per directory navigation; no re-query on every render cycle
- New `src/git.rs` module with pre-computed dirty-dir set for O(1) `subtree_dirty()` checks

## [0.2.2] - 2026-03-23

### Added
- `rust-toolchain.toml` pinning the channel to `stable` for consistent builds across CI and contributors
- MSRV (`rust-version = "1.80"`) in `Cargo.toml` so users get a clear error on outdated toolchains

## [0.2.1] - 2026-03-23

### Added
- AGENTS.md documenting contribution process, version bumping rules, branch naming, and commit conventions
- CHANGELOG.md to track all changes going forward
- Demo GIF generated with VHS showing navigation, file preview, and fuzzy search
- Issue templates for bug reports and feature requests
- PR template
- CI and release GitHub Actions workflows
- Homebrew tap support (`brew install bradleyfay/trek/trek`)

## [0.2.0] - 2026-03-23

### Added
- Nerd font icons for 100+ file extensions and special directories
- Fuzzy search with `/` — incrementally filters files in the current directory
- Yank to clipboard via OSC 52: `y` copies relative path, `Y` copies absolute path
- Mouse-resizable panes — drag dividers to adjust column widths
- Scroll wheel support for scrolling file previews
- Shell integration (`trek --install-shell`) installs an `m` function that `cd`s on quit
- Help overlay with `?`
- Toggle hidden files with `.`
- `~` key to jump to home directory
- Homebrew tap: `brew install bradleyfay/trek/trek`
- CI workflow (fmt, clippy, test, build on every PR)
- Release workflow (tag-triggered, builds arm64 + x86_64, auto-updates tap formula)
- MIT license

### Changed
- Renamed project from `murdoch` to `trek`

## [0.1.0] - 2021-02-22

### Added
- Initial three-pane file manager (parent / current / preview)
- Vim-style keyboard navigation (h/j/k/l)
- Mouse click to select and enter directories
