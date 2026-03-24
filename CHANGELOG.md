# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
