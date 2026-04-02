# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Performance

- **Faster directory sorting** — `sort_entries` now uses `sort_by_cached_key` instead of `sort_by`, computing `.to_lowercase()` and extension keys once per entry (O(n)) rather than once per comparison (O(n log n)). Eliminates up to millions of `String` allocations when navigating large directories.
- **Non-blocking git status** — git status, branch detection, and gitignore filtering no longer run synchronously on the UI thread. All three git subprocesses (`rev-parse`, `branch --show-current`, `status --porcelain`) are now dispatched to a background thread via an `mpsc` channel, matching the existing async preview pattern. Navigation remains fully responsive while git status loads; decorations update on the next event-loop tick. The `R` key (manual refresh) follows the same async path.
- **Instant startup** — `App::new` no longer blocks on `git rev-parse --show-toplevel` before rendering the first frame. The recursive change-feed watcher starts on `cwd` immediately and is repointed to the true git repo root once the first async git-status result arrives.

### Performance

- **Cached hex-tool probe** — `xxd` / `hexdump` availability is now probed exactly once per session via a `std::sync::LazyLock`, instead of shelling out to `which` on every hex-preview render. The probe spawns the binary directly (no `which`), so it works even when `which` is absent.

### Fixed

- **No clipboard popups during tests** — `osc52_copy` now checks `IsTerminal` before writing the OSC 52 escape sequence. Prevents macOS clipboard-access permission dialogs when running `cargo test`.

## [0.65.0] - 2026-04-01

### Added

- **Preview focus mode** — pressing `→` or `l` on a **file** now moves the cursor into the preview pane instead of opening the file in an editor. The preview border turns cyan to signal focus. While focused:
  - `j` / `↓` and `k` / `↑` move a highlighted cursor line-by-line through the file
  - `J` / `K` extend a selection range (anchor set on first press; selected lines highlighted in dark gray)
  - `g` / `G` jump to the first / last line
  - `[` / `]` still scroll the pane
  - `Enter` opens the file in `$EDITOR` and exits focus
  - `←`, `h`, or `Esc` return focus to the file tree
  - Layout toggles (`\`, `w`, `#`, `U`, `?`, `q`) continue to work while the preview is focused

- **Send lines to cmux surface** (`Tab` in preview focus) — with one or more preview lines highlighted, pressing `Tab` opens a surface picker overlay showing all terminal, browser, and markdown surfaces in the current cmux workspace. Type to filter, `↑`/`↓` to navigate, `Enter` to send. The selected text is pasted into the target surface and cmux immediately shifts keyboard focus to that pane — your next keystroke lands there, not back in Trek. Designed for AI-assisted development: select relevant code in Trek, Tab, pick your Claude session, and start typing your follow-up question without any manual copy-paste or window switching.

## [0.64.0] - 2026-04-01

### Added
- **Toggle left parent-directory pane** (`\`) — press `\` to collapse the left pane to zero width, giving the current-directory listing and preview pane the full terminal width. Press `\` again to restore it. The pane's previous width is saved and restored exactly, including any custom drag-resized widths. Accessible via the command palette as "Toggle parent pane (hide/show left pane)".

## [0.63.0] - 2026-03-31

### Added
- **AI context bundle builder** (`Ctrl+B`) — select files and export them as a formatted Markdown bundle to the clipboard, ready to paste into an AI chat. Supports three formats: paths only (`p`), paths + file contents (`c`), and paths + git diff (`d`). Accessible via command palette and the `?` help overlay. Bundles over 512 KB prompt for confirmation before copying.

### Fixed
- **Animated GIF preview** — `chafa` now receives `--animate=off` so only the first frame is rendered as a static image, preventing GIF corruption in the preview pane.

### Docs
- Documented image preview (raster formats, inline `chafa` rendering, SVG as text) and added `chafa` to the optional dependencies table.

## [0.62.1] - 2026-03-27

### Changed
- **Markdown and HTML files reuse existing viewer surfaces instead of opening new panes** — when double-clicking (or pressing `l`/`Enter` on) a `.md`/`.markdown` file, Trek now calls `cmux list-surfaces --json` to check whether a markdown viewer surface is already open. If one is found, the file opens as a new tab inside that surface (`cmux markdown open --surface <id>`). If none is found, a fresh surface is created as before. The same logic applies to HTML files: an existing browser surface receives a new tab (`cmux browser <id> tab new <url>`), otherwise `cmux browser open` creates one. This prevents cmux workspaces from filling up with duplicate viewer panes during a session.

## [0.62.0] - 2026-03-27

### Changed
- **HTML files now open in the cmux browser by default** — `.html` and `.htm` files previously routed to the system default opener (`open` / `xdg-open`). They now open in the cmux embedded browser via `cmux browser open {}`, keeping the rendered view inside the cmux workspace alongside Trek and the editor. Images, PDFs, and other binary types continue to use the system opener. Users with a custom `opener.conf` are unaffected.

## [0.61.3] - 2026-03-27

### Fixed
- **Correct cmux markdown command** — the built-in default rule for `.md`/`.markdown` files was using `cmux open --md {}`, which is not a valid cmux command. The correct invocation is `cmux markdown open {}`. Updated the default rule, the README example, and the module doc comment. Users with a custom `opener.conf` are unaffected.

## [0.61.2] - 2026-03-27

### Fixed
- **Markdown files now open in the cmux viewer by default** — double-clicking or pressing `l`/`Enter` on a `.md` or `.markdown` file was incorrectly falling through to `$EDITOR` instead of the cmux markdown viewer. The built-in default opener rules were missing a markdown entry; `.md`/`.markdown` files now correctly route to `cmux markdown open {}` when no `opener.conf` is present. Users with a custom `opener.conf` are unaffected.

## [0.61.1] - 2026-03-24

### Fixed
- **Trek now opens in the invocation directory** (`$PWD`) instead of restoring the last session's working directory. Previously, running `trek` without an argument would restore the `cwd` saved from the previous session, causing different workspace panes to all show the same directory. The fix adds `std::env::current_dir()` to the start-directory fallback chain so the priority is: explicit path arg → shell's `$PWD` → saved session directory (last resort, for when `$PWD` no longer exists).

## [0.61.0] - 2026-03-24

### Added
- **Session change summary** (`Ctrl+S`) — answers "what changed during this conversation?" with a cross-file diff between a filesystem snapshot and the current state. The center pane shows all files grouped as **NEW**, **MODIFIED**, and **DELETED** since the checkpoint, with file sizes and byte deltas. The checkpoint is taken lazily on first open; press `C` inside the summary to reset it to now (e.g., at the start of a new AI session), or `R` to refresh without moving the baseline. `j`/`k` navigate the list; `l`/`Enter` exits summary mode and jumps directly to that file in the tree; `Esc` returns to normal navigation. Respects gitignore state and always includes hidden files so mid-session toggles don't create gaps. Capped at 200 entries for performance. Accessible from the command palette as "Session summary" and "Reset session checkpoint". The feature is implemented in a new, self-contained `session_snapshot` module.

## [0.60.0] - 2026-03-24

### Added
- **Archive virtual-filesystem browser** — pressing `l` or `Enter` on any archive file (`.zip`, `.jar`, `.war`, `.ear`, `.tar`, `.tar.gz`, `.tgz`, `.tar.bz2`, `.tar.xz`, `.tar.zst`) enters a virtual directory browser. The archive contents are presented as a navigable three-pane tree identical to the regular filesystem view: directories appear first, files below. Navigate with the usual `j`/`k`/`h`/`l` keys. `l`/`Enter` on a virtual directory steps into it; `h`/Left steps back out; `l`/`Enter` on a file extracts it to a temp directory and shows its preview; `Esc` exits archive mode and returns to the real filesystem. The path bar shows a breadcrumb (`archive.zip / src / utils`) with a hint while in archive mode. Zip-family archives use the bundled `zip` crate with no external dependencies; tar archives delegate to the system `tar` command. The feature is implemented as a new, self-contained `archive_nav` module.

## [0.59.0] - 2026-03-24

### Added
- **Background file operations with task manager** — copy, move, and archive extraction now run on background threads so Trek remains fully interactive during large transfers. A task manager panel (`Ctrl+T`, also accessible via the command palette) lists all active and recently completed operations with their status (⟳ running / ✓ done / ✗ failed), operation type, label, and elapsed time. Press `j`/`k` to navigate, `c` to clear completed tasks, and `Esc`/`q` to close. The panel is also shown in the preview pane area (replaces the preview while open). Status-bar messages now include a `(Ctrl+T to monitor)` hint when a background op is in flight.

### Removed
- The previously blocking synchronous `paste_clipboard` and `confirm_extract` methods have been replaced by their asynchronous equivalents and removed. All paste/extract actions are now non-blocking.

## [0.58.0] - 2026-03-24

### Added
- **Image and PDF preview** — raster image files (`.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.ico`, `.webp`, `.avif`, `.tiff`) and `.pdf` files now display a rich metadata card in the preview pane instead of the previous `[binary file]` placeholder. The card shows format, file size, and pixel dimensions (for images) or PDF version (for PDFs). When [`chafa`](https://hpjansson.org/chafa/) is installed, images are additionally rendered as full-color Unicode/sixel art inline in the preview pane at 72 columns. When [`pdfinfo`](https://poppler.freedesktop.org/) (poppler-utils) is installed, PDFs display their full document metadata. Both tools degrade gracefully when absent — a short install hint is shown instead. SVG files continue to preview as plain-text XML through the existing text path.

## [0.57.0] - 2026-03-24

### Added
- **Async non-blocking preview rendering** — the preview pane now loads file contents, git diffs, directory listings, hex dumps, and all other preview modes on a background thread. Trek remains fully interactive while large files highlight or diffs are computed. A `"Loading…"` placeholder is shown immediately while the background job runs. Navigating to a new file while a preview is still in flight automatically cancels the in-progress render and starts a fresh one — no stale results are ever displayed.

## [0.56.1] - 2026-03-24

### Fixed
- **TOML syntax highlighting** — `.toml` files (e.g. `Cargo.toml`) now render with full syntax highlighting in the preview pane. The bundled syntax set has been switched from Sublime Text's default package (which lacks TOML) to the `two-face` extended set — the same set used by `bat` — which includes TOML, Dockerfile, `.env`, and dozens of other types not covered by the defaults. `.yaml`, `.json`, `.rs`, and all previously-supported types are unaffected.

## [0.56.0] - 2026-03-24

### Added
- **Live change feed** (`F`) — press `F` to open a real-time overlay in the preview pane area showing every filesystem event under the project root (detected via `git rev-parse --show-toplevel`, falling back to Trek's launch directory). Events are listed most-recent-first with a relative age (`mm:ss`), a kind symbol (`+` created, `~` modified, `✕` deleted), and a path relative to the project root. Navigate with `j`/`k`, jump to a file with `Enter`/`l`, clear the buffer with `c`, and close with `F` or `Esc`. The buffer holds up to 500 events; oldest are evicted when full. Build artifact directories (`.git/`, `target/`, `node_modules/`, `__pycache__/`, `.next/`, `dist/`, `build/`) are suppressed from the feed. The feed header shows `(paused)` when watch mode is disabled (`I`).
- **Toggle change feed** action added to the command palette as `"Toggle change feed (live filesystem event stream)"`.

### Changed
- **Clipboard inspector keybinding moved** from `F` to `F9` — `F` is now reserved for the change feed. The action remains accessible via the command palette as `"Inspect clipboard contents"` and the help overlay is updated accordingly.

## [0.55.0] - 2026-03-24

### Added
- **Right-click to open in new cmux tab** — right-clicking a file in the current pane selects it and opens it in a new cmux tab using the same file-type routing as `l`/`Enter` (opener config or built-in defaults).
- **Double-click to open to the right** — double-clicking a file opens it in a new cmux terminal pane split to the right of Trek's pane (`cmux new-pane --direction right`). Falls back to spawning the system opener for non-editor file types. Both actions show a status-bar hint when Trek is not running inside cmux.
- Both actions are listed in the help overlay (`?`) and command palette.

## [0.54.0] - 2026-03-24

### Added
- **Rifle-style configurable file opener** — Trek now reads `~/.config/trek/opener.conf` (or `$XDG_CONFIG_HOME/trek/opener.conf`) and evaluates rules top-to-bottom; the first match wins. Rules use `ext <ext1|ext2>` or `glob <pattern>` matchers with a `{}` path placeholder in the command. Example: `ext md : cmux markdown open {}`.
- Built-in defaults ship as the fallback when no config file is present, preserving existing routing behaviour (system open for HTML/images/PDFs, `$EDITOR` in a new cmux surface for code/text).
- User-configured commands are executed via `sh -c`, allowing full shell syntax and environment variable expansion.

## [0.53.0] - 2026-03-24

### Removed
- **Bulk rename with regex** (`r`) — complex file operations belong in the AI assistant (Claude Code, Codex), not in Trek. Single-file quick rename (`n` / `F2`) is unaffected.
- **Archive creation** (`E`) — creating archives is an AI-delegable operation. Archive extraction (`Z`) and browsing are unchanged.
- **SHA-256 hash preview** (`H`) — niche sysadmin feature that doesn't fit Trek's inspection-focused purpose.
- **Glob pattern selection** (`*`) — removed alongside bulk rename, which was its primary consumer.

## [0.52.0] - 2026-03-24

### Added
- **cmux tab open**: pressing `l`, `→`, or `Enter` on a file now opens it in a new cmux tab instead of yanking its path to the clipboard
- File type routing: HTML/images/PDFs open with the system default opener; all other text/code files open in `$EDITOR` inside a new terminal surface
- Falls back gracefully with a status-bar hint when Trek is not running inside cmux
- `Open file in new cmux tab` action added to the command palette (`l / Right / Enter`)

### Changed
- `enter_selected` on a file now calls `open_in_cmux_tab` — consistent with the "right means go deeper / act on this" navigation model
- `l`, `→`, and `Enter` on a file no longer silently yank the relative path; use `y` to yank

## [0.51.0] - 2026-03-24

### Added
- **Always-on filesystem watcher**: Trek now automatically watches the current directory for changes using OS-native events (FSEvents on macOS, inotify on Linux) via the `notify` crate — no keypress required
- The listing refreshes automatically within ~150 ms of any file creation, deletion, rename, or modification in the current directory
- `I` still toggles the watcher off/on for users who prefer manual refresh; the `[watch]` badge in the path bar reflects the active state
- Watcher updates automatically when navigating to a new directory
- Graceful degradation: if the OS watcher fails to start (e.g. inotify limit, read-only filesystem), Trek continues working exactly as before with manual `R` refresh
- Debounce window of 150 ms coalesces rapid bursts (e.g. `git checkout` touching many files) into a single reload

### Changed
- Watch mode is now **on by default** — previously required pressing `I` to activate
- The event loop polls with a 150 ms timeout (down from 500 ms) when the watcher is active, driven by OS events rather than mtime polling

## [0.50.0] - 2026-03-24

### Added
- **Session restore**: Trek now remembers where you were when it last exited and reopens there on the next launch (no argument required)
- Restores the current directory, the selected entry (by name, so it survives renames of other files), `show_hidden` state, sort mode, and sort order
- Session file lives at `$XDG_DATA_HOME/trek/session` (default `~/.local/share/trek/session`) using the same XDG pattern as bookmarks
- Explicit path argument (`trek /some/path`) always wins — session is never restored when a start directory is given
- Missing or deleted saved directory falls back silently to CWD; corrupt/absent session file causes no error
- Session is written only on clean `q`/`Q` exit — panic or SIGKILL leaves the previous session intact
- All session fields are forward-compatible: unknown keys from future versions are silently ignored

## [0.49.0] - 2026-03-24

### Added
- **Tab completion in path-jump bar**: press `e` to open the jump bar, then `Tab` to complete the current path prefix using filesystem entries
- Single match: completes to the full name and appends `/` for directories
- Multiple matches: advances to the longest common prefix of all matching entries
- No matches: input unchanged (silent no-op)
- `~` prefix expands to the home directory for completion, then the `~` is preserved in the displayed input
- Hint text in the jump bar updated to show `Tab=complete  Enter=go  Esc=cancel`
- Help overlay and command palette updated to reflect Tab completion

## [0.48.0] - 2026-03-24

### Added
- **`E` — create archive from selected files**: press `E` to open a filename input bar (pre-filled with `<entry>.tar.gz`) and type an archive name; Trek infers the format from the extension and runs the appropriate tool
- Supports `.tar.gz` / `.tgz`, `.tar.bz2` / `.tbz2`, `.tar.xz` / `.txz`, `.tar.zst` / `.tzst`, `.tar`, and `.zip`; `.gz` and `.7z` are unsupported for creation and show a clear error
- `E` with no selection archives the current entry; with 1 selection it pre-fills `<name>.tar.gz`; with multiple selections it pre-fills `archive.tar.gz`
- Created archive appears in the listing immediately with the cursor moved to it
- Error cases handled gracefully: unknown extension, output already exists, `zip` binary not found
- `BeginArchiveCreate` registered in command palette (`E`) and `?` help overlay
- `Z` extracts; `E` creates — they are a symmetric pair

## [0.47.0] - 2026-03-24

### Added
- **`I` — watch mode**: press `I` to toggle watch mode, which auto-refreshes the directory listing whenever the filesystem detects a change to the current directory
- Uses `crossterm::event::poll` with a 500 ms timeout so the event loop yields frequently enough to catch directory mtime changes without busy-waiting
- On toggle-on, records the current directory's mtime as a baseline; on toggle-off, clears it
- On detecting a change, re-runs `load_dir()` and attempts to restore the previously selected entry by name so the cursor doesn't jump unexpectedly
- Status bar shows `"Watch mode ON — listing auto-refreshes on changes"` / `"Watch mode OFF"` on toggle
- Path bar gains a cyan `[watch]` badge when watch mode is active
- `ToggleWatchMode` registered in the command palette (`I`) and `?` help overlay

## [0.46.0] - 2026-03-24

### Added
- **`D` — disk usage preview**: press `D` on a directory to see its immediate children sorted largest-first with human-readable sizes and proportional Unicode block bars
- Preview pane title shows `dirname [du]` when disk usage mode is active
- Uses `du -k -d 1` (POSIX-compatible; works on macOS BSD `du` and Linux GNU `du`)
- `D` on a file shows `"Disk usage view is for directories"` and does nothing
- Empty directories show `"(empty directory)"`; graceful error if `du` is not found
- Mutually exclusive with all other special preview modes (hash, hex, compare, meta, git log, diff)
- `ToggleDuPreview` registered in the command palette (`D`) and `?` help overlay

## [0.45.0] - 2026-03-24

### Added
- **Session persistence**: Trek now saves `cwd` and all marks to `~/.local/share/trek/session` (or `$XDG_DATA_HOME/trek/session`) on clean exit, and restores them on the next launch
- Marks (`\`a`–`\`z`, `\`A`–`\`Z`) survive restarts — no need to re-navigate and re-set them
- If Trek is launched with an explicit path argument, the saved cwd is ignored (explicit always wins)
- Saved paths that no longer exist are silently skipped — Trek starts without them
- Write failures on quit are non-fatal — Trek never crashes due to a failed session save
- Follows the same XDG-aware, no-new-dependencies pattern as `src/bookmarks.rs`

## [0.44.0] - 2026-03-24

### Added
- **`Z` — archive extraction**: press `Z` on any recognized archive to extract it into the current directory
- Supported formats: `.tar`, `.tar.gz`/`.tgz`, `.tar.bz2`/`.tbz2`, `.tar.xz`/`.txz`, `.tar.zst`/`.tzst`, `.zip`/`.jar`/`.war`/`.ear`, `.gz`, `.7z`
- A confirmation bar shows `[ Extract ] "filename" → ./ [y/Enter · Esc to cancel]` before acting
- On success: listing refreshes and status bar shows `"Extracted: <name>"`
- On failure: status bar shows `"Extract failed: <first stderr line>"`
- `Z` on a non-archive file shows `"Not an archive"` and does nothing
- Graceful error messages when `unzip` or `7z` are not installed
- `BeginExtract` registered in the command palette (`Z`) and `?` help overlay

## [0.43.0] - 2026-03-24

### Added
- **`a` — hex dump preview**: press `a` to toggle a hex dump of the selected file in the preview pane using `xxd` (or `hexdump -C` as fallback)
- Preview pane title shows `filename [hex]` when hex view is active
- Files larger than 4 MB show a size-limit message instead of attempting a full dump
- Graceful fallback message when neither `xxd` nor `hexdump` is found
- Works on any file type — text or binary; blocked for directories (status message shown)
- Mutually exclusive with all other special preview modes (hash, meta, git log, diff, compare)
- `ToggleHexView` registered in the command palette (`a`) and `?` help overlay

## [0.42.0] - 2026-03-24

### Added
- **`f` — two-file compare**: select exactly 2 files with `Space`/`J`/`K`, then press `f` to show a unified diff of the two files in the preview pane
- Preview pane title shows `<file1> ↔ <file2> [compare]` when compare mode is active
- Uses `diff -u` (POSIX); shows `(files are identical)` when there are no differences
- Requires exactly 2 non-directory entries to be selected; shows a status message otherwise
- Mutually exclusive with all other special preview modes (diff, meta, hash, git log)
- `CompareFiles` registered in the command palette (`f`) and `?` help overlay

## [0.41.0] - 2026-03-24

### Added
- **`m` meta card — line/word/char counts**: the meta preview for regular text files now appends `Lines`, `Words`, and `Chars` rows after `Accessed` — equivalent to `wc -l -w -m` inline
- Binary files (non-UTF-8) and files over 10 MB silently omit the stats rows
- Directories and symlinks are unaffected

## [0.40.0] - 2026-03-24

### Added
- **`V` — git log preview**: press `V` to toggle `git log --oneline -30 -- <path>` in the preview pane for the selected file or directory
- Works for directories too — shows commits that touched any file in the subtree
- Preview pane title shows `[log]` when git log mode is active
- Gracefully degrades outside git repos: shows `"(git log failed — not a git repository?)"` or `"(no commits for this path yet)"`
- Mutually exclusive with diff (`d`), meta (`m`), and hash (`H`) preview modes
- `V` scrollable with `[`/`]` like all other preview content
- `ToggleGitLogPreview` registered in the command palette and `?` help overlay

## [0.39.0] - 2026-03-24

### Added
- **`z` — frecency jump list**: press `z` to open a session-scoped overlay listing recently visited directories ranked by frecency (frequency × recency)
- Overlay auto-populates as you navigate; no manual setup required
- Score = `visits × recency_weight` where weight is 4× (< 1 hr), 2× (< 24 hr), 1× (< 1 week), 0.5× (older)
- Type to filter by directory name; `Enter` jumps, `Esc`/`z` closes
- Yellow border and highlight to distinguish from bookmarks (cyan)
- Stale entries (directory deleted) show an error message rather than crashing
- `OpenFrecency` registered in the command palette and `?` help overlay

## [0.38.0] - 2026-03-24

### Added
- **Symlink target and validity in meta preview (`m`)**: the meta card for a symlink now shows two additional lines immediately after `Type`:
  - `Target` — the raw stored link path (relative targets shown as-is; `$HOME` replaced with `~`)
  - `Valid  ✓  exists` or `Valid  ✗  dangling` depending on whether the full symlink chain resolves
- A permission error reading the link shows `Target    (unreadable)` with no `Valid` line
- Regular file and directory meta cards are unchanged

## [0.37.0] - 2026-03-24

### Added
- **Selection total size in status bar**: when one or more files are selected, the bottom bar now shows the aggregate byte size alongside the count — e.g., `" 3 selected  (2.4 MB)"`
- Directories in the selection do not contribute to the total (their `DirEntry.size` is a meaningless filesystem block size)
- When only directories are selected the size annotation is omitted entirely
- `C` (copy-selected) status message now also includes total file size: `"[copy] 3 files (2.4 MB)"`

## [0.36.0] - 2026-03-24

### Added
- **`F` — clipboard inspector**: press `F` to open an overlay showing the current clipboard contents (paths queued for copy or cut)
- Overlay title and border are green for copy operations, yellow for cut operations, and grey when clipboard is empty
- Press `p` inside the inspector to close and immediately paste; press `Esc` or `F` to close without action
- `InspectClipboard` registered in the command palette as "Inspect clipboard contents" and in the `?` help overlay

## [0.35.0] - 2026-03-24

### Added
- **`N` — directory item counts**: directory entries now show child item counts (`"12 items"`, `"0 items"`, `"  1 item"`, `">1000 items"`) by default instead of meaningless filesystem block sizes
- Press `N` to toggle back to raw block sizes; press `N` again to restore counts
- Counting uses `read_dir().take(1001)` capped at 1001 to prevent UI hangs on huge directories — directories with >1000 items display `">1000 items"`
- Unreadable directories (permission denied) display `"? items"`
- `show_timestamps` (key `T`) takes priority over counts — when timestamps are active, all entries show dates regardless of `show_dir_counts`
- `N` registered in the command palette as "Toggle directory item counts" and in the `?` help overlay

## [0.34.0] - 2026-03-24

### Added
- **`U` — preview word wrap**: press `U` to soft-wrap long lines at the preview pane boundary; press `U` again to restore truncated rendering
- Preview pane title shows `[wrap]` indicator when wrap mode is active (alongside existing `[diff]`, `[meta]`, `[hash]` indicators)
- `preview_wrap` composes with `show_line_numbers` — both can be active simultaneously
- Wrap applies to all preview content types: plain text, syntax-highlighted source, diff, meta card, hash card
- `U` registered in the command palette as "Toggle preview word wrap" and in the `?` help overlay

## [0.33.0] - 2026-03-24

### Added
- **`T` — listing timestamps**: press `T` to replace the file-size column with compact last-modified dates; press `T` again to return to sizes
- Same-year files show `"Jan 15 14:32"` (12 chars); prior-year files show `"2023 Nov  8 "` (12 chars); unavailable timestamps show `"----  --:--"`
- Fixed 12-character column width ensures layout stability across all date formats
- Directories show no annotation in timestamp mode (consistent with size-column behaviour)
- `show_timestamps` persists across directory navigation within the session
- `T` registered in the command palette as "Toggle modification timestamps in listing" and in the `?` help overlay
- Date arithmetic uses Trek's existing `is_leap_year` helper — no new crate dependencies

## [0.32.0] - 2026-03-24

### Added
- **`w` — preview pane collapse**: press `w` to hide the right preview pane entirely, expanding the centre listing column to full width; press `w` again to restore the pane to its previous divider position
- Collapse saves the current `right_div` ratio (including any user-dragged position) and restores it exactly on expand
- `w` registered in the command palette as "Toggle preview pane (hide/show right pane)" and in the `?` help overlay

## [0.31.0] - 2026-03-24

### Added
- **`H` — hash preview**: press `H` on any file to display its SHA-256 checksum card in the preview pane; shows the full 64-hex-character hash, filename, and human-readable file size
- Uses `shasum -a 256` (macOS) or `sha256sum` (GNU coreutils); shows a helpful install hint if neither is available
- Files larger than 512 MB show a size-limit message instead of blocking the UI
- `H` on a directory shows `"Hash preview not available for directories"` in the status bar without entering hash mode
- `hash_preview_mode` is mutually exclusive with `meta_preview_mode` and `diff_preview_mode`; activating any of the three clears the others
- `hash_preview_mode` is cleared on directory navigation (same behaviour as `meta_preview_mode`)
- `H` appears in the command palette as "Toggle hash preview (SHA-256 checksum)" and in the `?` help overlay alongside `d` and `m`

## [0.30.0] - 2026-03-24

### Added
- **`L` — create symlink**: press `L` on any file or directory to open a `Symlink → <target> :` bar (LightBlue label) pre-filled with the entry's name; press Enter to create a symbolic link at `cwd/<name>` pointing to the selected entry's absolute path; the listing refreshes and the cursor selects the new symlink
- Uses `symlink_metadata().is_ok()` (not just `.exists()`) to detect dangling symlinks that `.exists()` misses — prevents accidental overwrites of broken link names
- `git_status` refreshed after creation so newly created symlinks appear with correct status indicators
- Error cases: empty name → "Symlink name cannot be empty"; destination exists → "'<name>' already exists"; other OS error → "symlink failed: <message>"
- `L` on empty directory is a no-op (bar does not open)
- Non-Unix platforms: confirming shows "Symlink creation requires a Unix system" instead of panicking (`#[cfg(unix)]` guard)
- Completes Trek's file-creation suite: `M` (directory), `t` (file), `W` (duplicate), `L` (symlink)
- `L` registered in the command palette as "Create symlink to selected entry"
- `L` documented in help overlay (`?`) under File Operations and in `--help` output
- 7 new BDD-style unit tests; all 187 tests pass

## [0.29.0] - 2026-03-24

### Added
- **`` ` `` / `'` — per-session directory marks**: press `` ` `` then a letter (`a`–`z`, `A`–`Z`) to record the current directory to that slot; press `'` then the same letter to jump back instantly from anywhere in the session
- Marks are session-only (in-memory `HashMap<char, PathBuf>`) — they are never written to disk; the persistent bookmark system (`b`/`B`) handles cross-session locations
- 52 available slots (`a`–`z` and `A`–`Z`); re-marking a slot silently overwrites it
- `` ` `` followed by `Esc` or a non-letter cancels silently; `'` followed by an unset letter shows `"Mark '<c>' not set"`; a set letter whose directory was deleted shows `"Mark '<c>' no longer exists"` without crashing
- Mark jumps push to the history stack so `Ctrl+O`/`Ctrl+I` navigation works correctly after a mark jump
- `filter_input` and `filter_mode` are cleared on mark navigation (consistent with all other navigation methods)
- Both actions registered in the command palette: "Set mark" and "Jump to mark"
- Both documented in help overlay (`?`) under Navigation and in `--help` output
- 8 new BDD-style unit tests; all 180 tests pass

## [0.28.0] - 2026-03-24

### Added
- **`A` — yank path format picker**: press `A` on any entry to open a compact four-option overlay just above the status bar; press a single mnemonic key to copy in the desired format via OSC 52 and close the picker
- Four formats: `r` (relative `./…`), `a` (absolute `/…`), `f` (filename only), `p` (parent directory path); numeric aliases `1`–`4` also accepted
- `strip_prefix` failure (e.g. symlink outside cwd) causes `r` to fall back to absolute path without panic; `parent()` returning `None` (root `/`) copies `/` without panic
- Paths longer than 44 characters are truncated with `…` in the overlay display; the **full untruncated string** is what gets copied to the clipboard
- Existing `y` (relative) and `Y` (absolute) direct bindings are unchanged
- `A` on an empty directory is a no-op (picker does not open)
- `Esc` dismisses the picker without copying or setting a status message; any unrecognised key is silently ignored while the picker stays open
- `A` registered in the command palette as "Yank path (pick format: relative/absolute/filename/parent)"
- `A` documented in help overlay (`?`) under Yank & Misc and in `--help` output
- New methods: `yank_filename`, `yank_parent_dir`, `open_yank_picker`, `close_yank_picker` in `src/app/yank.rs`
- 6 new BDD-style unit tests; all 173 tests pass

## [0.27.0] - 2026-03-24

### Added
- **`W` — duplicate entry in place**: press `W` on any file or directory to open a `Duplicate:` bar (Cyan label) pre-filled with a suggested name; edit the name and press Enter to copy the entry to `cwd/<name>`; the listing refreshes and the cursor moves to the new entry
- Name suggestion algorithm uses the first dot to preserve compound extensions: `archive.tar.gz` → `archive_copy.tar.gz`; `config.toml` → `config_copy.toml`; `Makefile` → `Makefile_copy`
- Destination already existing shows `'<name>' already exists` without touching the filesystem; empty name shows "Name cannot be empty"
- Directories are duplicated recursively via the existing `ops::copy_path` (which already handles recursive directory copy)
- Success message: `Duplicated → "<name>"`; after success the new entry is selected in the refreshed listing
- `W` registered in the command palette as "Duplicate entry in place"
- `W` documented in help overlay (`?`) under File Operations and in `--help` output
- 8 new BDD-style unit tests covering: open bar with suggested name, cancel, file copy success, existing-name error, empty-name error, compound extension, no-extension, empty-directory noop

## [0.26.0] - 2026-03-24

### Added
- **`*` — glob pattern selection**: opens an inline `Glob select:` input bar at the bottom (Magenta label); typing a glob pattern (e.g. `*.rs`, `*.log`, `test_?`) and pressing Enter adds all matching files in the current directory to the rename selection set (`rename_selected`); union semantics — multiple patterns can be chained without losing prior selections
- `*` matches any sequence of characters (including empty); `?` matches exactly one character; all other characters (including `.`) are regex-escaped so `*.tar.gz` matches literal dots, not "any character"
- Non-matching patterns display "No entries match: `<pattern>`" in the status bar; the bar closes regardless; directories are always excluded from matches
- Empty pattern silently closes the bar (no-op); malformed patterns (invalid after glob→regex conversion) display "Invalid glob pattern: `<pattern>`"
- Glob→regex conversion implemented as `glob_to_regex()` (private free function in `src/app/rename.rs`) using `regex::escape` for non-metacharacters — no new crate dependencies
- `*` registered in the command palette as "Select files by glob pattern"
- `*` documented in help overlay (`?`) under Selection & Rename and in `--help` output
- 8 new unit tests: open bar, cancel without selecting, extension match, no-match message, union-add semantics, empty pattern noop, `?` single-char match, literal-dot handling

## [0.25.0] - 2026-03-24

### Added
- **`#` — preview line numbers**: toggles a numbered gutter in the preview pane; each line is prefixed with its 1-based absolute line number right-justified in a dynamic-width field (`total.to_string().len()` digits) followed by ` │ `; gutter text is `Color::DarkGray` to stay visually recessive
- Line numbers are correct regardless of scroll offset — the plain-content path uses `enumerate()` before `skip()` for absolute indices; the highlighted path computes `preview_scroll + i + 1` after `skip()` followed by `enumerate()`
- The toggle persists across file navigation (`show_line_numbers` is a session-level display preference, like `show_hidden`)
- Works in all preview modes: raw content, syntax-highlighted source, git diff output, metadata card, and directory child listings
- Status bar shows "Line numbers: on" / "Line numbers: off" on each toggle
- `#` registered in the command palette as "Toggle line numbers in preview pane"
- `#` documented in help overlay (`?`) under View and in `--help` output
- 4 new unit tests: default off, toggle on, toggle off, persists across navigation

## [0.24.0] - 2026-03-24

### Added
- **`t` — touch / new file**: opens an inline `New file:` input bar at the bottom; typing a filename and pressing Enter creates a new empty file in the current directory using `create_new(true)` (atomic, no silent overwrites); the listing refreshes and the cursor selects the newly created file; status bar shows `Created "filename"`
- Empty filename shows "File name cannot be empty" and closes the bar; attempting to create a file that already exists shows `"'name' already exists"` without overwriting; other filesystem errors are surfaced as readable status messages
- `t` in the `pending_delete` confirmation branch still confirms trash — no conflict, as that branch runs before normal mode
- New `pub fn touch_file(parent: &Path, name: &str) -> Result<PathBuf>` in `src/ops.rs` using `OpenOptions::create_new(true)` for atomicity
- `t` registered in the command palette as "New file (touch — create empty file)"
- `t` documented in help overlay (`?`) under File Operations alongside `M` and in `--help` output
- 6 new unit tests: open bar, cancel without creating, create and select, empty name error, existing file error, push/pop char

## [0.23.0] - 2026-03-24

### Added
- **`[` / `]` — preview pane keyboard scrolling**: scrolls the preview pane up or down 5 lines per keypress in normal mode; complements the existing mouse-wheel scroll (3 lines per event) and gives keyboard-only users full access to any file longer than the visible pane height
- Works in all preview modes: raw content, syntax-highlighted content, diff preview (`d`), file metadata view (`m`), and directory child listings
- `[` clamps at scroll offset 0 (no underflow); `]` clamps at `preview_lines.len() - 1` (no overshoot); both are no-ops on empty previews
- Navigating to a different file resets preview scroll to 0 (existing behaviour, no regression)
- `[`/`]` registered in the command palette as "Scroll preview pane up/down"
- `[`/`]` documented in help overlay (`?`) under Navigation and in `--help` output
- 5 new unit tests: down advances offset, up decreases offset, top clamps at 0, bottom clamps at max, empty preview is no-op

## [0.22.0] - 2026-03-24

### Added
- **`J` / `K` — range selection**: pressing `J` (Shift+j) marks the current entry as selected, moves the cursor down, and marks the new current entry; `K` does the same moving up; enables fast contiguous multi-file selection without pressing Space on each entry individually
- Both `J` and `K` include all entry types (files and directories) in the selection so bulk copy (`C`) and bulk delete (`X`) work naturally with directory entries; `start_rename` (`r`) filters out directories and shows an informative message if no files remain
- `J`/`K` stop at list boundaries without wrapping — the boundary entry is marked and the cursor stays
- Updated `start_rename()` guard: previously rejected an empty `rename_selected` set; now rejects selections that contain no *file* entries (e.g. only directories from range selection) with the message "No files selected (directories cannot be renamed in bulk)"
- `J`/`K` registered in the command palette under "Extend selection down/up (range select)"
- `J`/`K` documented in help overlay (`?`) under Selection & Rename and in `--help` output
- 6 new unit tests: down marks both endpoints, up marks both endpoints, bottom boundary stays and marks, top boundary stays and marks, directories included in range selection, only-dirs selection shows appropriate message in start_rename

## [0.21.0] - 2026-03-24

### Added
- **`e` — path jump bar**: opens a bottom input bar where the user can type any absolute path, relative path, or `~/…` path; `Enter` navigates there, `Esc` cancels; if the target is a file, trek navigates to its parent directory and places the cursor on the file
- Non-existent paths show an error in the status bar and leave the bar open for correction
- `e` registered in the command palette as `"Jump to path (path jump bar)"`
- `e` documented in help overlay (`?`) under Navigation and in `--help` output
- 7 new unit tests covering: open/cancel, empty-input silent cancel, absolute dir navigation, file path navigates to parent and selects file, nonexistent path error, push/pop char

## [0.20.0] - 2026-03-24

### Added
- **`i` — gitignore-aware listing**: toggles hiding of gitignored files and directories from the current listing; uses `git ls-files --others --ignored --exclude-standard --directory` scoped to the current directory so ignored directories are collapsed to a single entry rather than expanded; persists across directory navigation for the session (like `.` for hidden files)
- `[ignore]` badge appears in the path bar (yellow, bold) next to the git branch indicator when the filter is active
- Pressing `i` outside a git repository shows `"Not in a git repository"` and does nothing; git not on PATH degrades silently with no crash
- Filter is automatically re-applied on every `load_dir()` call (file operations, sort change, hidden-files toggle), consistent with `filter_input` behaviour
- New `src/app/gitignore.rs` module: `toggle_gitignored()`
- New `pub fn load_ignored(dir: &Path) -> HashSet<String>` in `src/git.rs`
- `i` registered in the command palette as `"Toggle gitignore filter (hide ignored files)"`
- `i` documented in help overlay (`?`) under View and in `--help` output
- 5 new unit tests: default-off, no-repo error message, `load_ignored` with real git repo, `load_ignored` outside repo returns empty, field toggle

## [0.19.0] - 2026-03-24

### Added
- **`n` / `F2` — quick single-file rename**: opens a lightweight inline bar at the bottom pre-filled with the current entry's full name; typing edits the name in place; `Enter` renames via `std::fs::rename`, refreshes the listing, and moves the cursor to the renamed entry; `Esc` cancels with no changes
- Empty input shows `"Name cannot be empty"` and closes the bar; name collision shows `"Already exists: <name>"` and closes the bar; same-name confirm is a silent no-op; works on both files and directories
- Status bar shows `Renamed "old" → "new"` on success
- `n` / `F2` registered in the command palette as `"Quick rename current file or directory"`
- Both keybindings documented in the help overlay (`?`) under Selection & Rename and in `--help` output
- 8 new unit tests covering: prefill, no-op on empty entries, cancel, same-name no-op, empty input error, successful rename, collision error, push/pop char

## [0.18.0] - 2026-03-24

### Added
- **`:` — command palette**: opens a centered searchable overlay listing all ~34 Trek actions with their keybinding hints; typing narrows the list with case-insensitive substring matching; `Enter`/`l` executes the highlighted action and closes the palette; `Esc`/`:` closes without executing; `j`/`k` and `Up`/`Down` navigate the list
- Empty query shows all actions; no-match query shows `"No matching actions"`; `Quit` appears for discoverability but is a no-op (use `q` directly)
- New `src/app/palette.rs` module: `ActionId` enum (34 variants), `PaletteAction` struct, `PALETTE_ACTIONS` static registry, `filter_palette()` function
- New `src/app/palette_ops.rs`: `open_palette`, `close_palette`, `palette_push_char`, `palette_pop_char`, `palette_move_up`, `palette_move_down`, `palette_selected_action`
- `execute_palette_action()` in `src/events.rs` dispatches all 34 action IDs; owns the terminal handle for future actions requiring TUI teardown
- `:` documented in help overlay (`?`) and `--help` output
- 9 new unit tests: filter empty/substring/no-match, open/close, push/pop char, navigation bounds, selected action ID

## [0.17.0] - 2026-03-24

### Added
- **`o` — open in terminal editor**: pressing `o` on a file tears down the TUI, launches `$VISUAL` → `$EDITOR` → `vi` (fallback) with the file path, then restores the TUI on exit; status bar shows `Returned from <editor>`; `app.load_dir()` is called on return to pick up filesystem changes; `o` on a directory is a no-op
- **`O` — open with system default**: spawns `open` (macOS) or `xdg-open` (Linux) in the background; Trek stays running; status bar shows `Opening <name> with system default…`; descriptive error shown if the opener is not available
- TUI is always restored after `o` even if the editor command fails to start (unconditional `enable_raw_mode` + `EnterAlternateScreen` outside the status match)
- Both bindings documented in the help overlay (`?`) under File Operations and in `--help` output
- 5 new unit tests covering `selected_file_path` (empty entries, directory no-op, file returns path) and `selected_path` (directory and file variants)

## [0.16.1] - 2026-03-24

### Changed
- **Refactor: decompose App god object** — `src/app.rs` (2,400 lines) converted to a `src/app/` module directory with 14 focused sub-files: `navigation.rs`, `layout.rs`, `mouse.rs`, `sort.rs`, `search.rs`, `rename.rs`, `file_ops.rs`, `content.rs`, `bookmarks.rs`, `find.rs`, `filter.rs`, `metadata.rs`, `yank.rs`, `preview.rs`
- `src/main.rs` split into three dedicated modules: `src/args.rs` (argument parsing + 9 unit tests), `src/shell.rs` (shell integration), `src/events.rs` (TUI event loop)
- Unit tests extracted from inline `mod tests` to `src/app/tests.rs`
- `src/app/mod.rs` reduced from ~2,400 lines to ~607 lines (type definitions + core lifecycle methods)
- No behaviour changes; all 96 tests pass

## [0.16.0] - 2026-03-24

### Added
- **`|` — filter/narrow mode**: opens an inline filter bar at the bottom; as the user types, the current directory listing narrows in real time to entries whose names contain the query (case-insensitive substring match)
- **`Enter` or `|` (in filter bar)**: closes the bar but keeps the filter active ("frozen"); the current pane title shows `dirname [~pattern]` to signal the active filter
- **`Esc` (in filter bar)**: clears the filter and closes the bar in one step, restoring the full listing
- **`Esc` (in normal mode, when a filter is active)**: clears the filter and restores the full listing; if no filter is active, clears selections as before
- Filter is automatically re-applied after every `load_dir` call (trash, paste, mkdir, sort change, hidden-files toggle), keeping the narrowed set coherent across file operations
- Navigating into a subdirectory, going to parent, going home, jumping via bookmark/find, or using history back/forward clears the filter (scoped to the current directory)
- `|` documented in the help overlay (`?`) under Search
- 8 new unit tests covering default state, start/close/clear, case-insensitive narrowing, pop-char widening, empty-match, and full-listing restoration

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
