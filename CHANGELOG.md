# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
