# Agent Guidelines for trek

This document describes how AI agents should contribute to trek. Follow these rules on every change — no exceptions.

---

## Every change requires three things

1. **A version bump** in `Cargo.toml`
2. **A CHANGELOG entry** in `CHANGELOG.md`
3. **A conventional commit message** with issue linkage when applicable

These must be in the same commit as the change itself. Never skip them.

---

## Version bumping (Semantic Versioning)

Use [semver](https://semver.org): `MAJOR.MINOR.PATCH`

| Change type | Bump |
|---|---|
| Breaking change (different keybindings, removed flag, changed behavior) | MAJOR |
| New feature, new option, new keybinding | MINOR |
| Bug fix, performance improvement, documentation, refactor | PATCH |

**Example:** fixing a crash → `0.2.0` → `0.2.1`. Adding a new command → `0.2.0` → `0.3.0`.

Update the version in `Cargo.toml`:
```toml
version = "0.3.0"
```

---

## CHANGELOG format

Add entries under `## [Unreleased]` at the top of `CHANGELOG.md`. Use these section headers as needed:

```markdown
## [Unreleased]

### Added
- New feature description

### Changed
- Changed behavior description

### Fixed
- Bug fix description

### Removed
- Removed feature description
```

When a release tag is pushed, rename `[Unreleased]` to `[x.y.z] - YYYY-MM-DD`.

---

## Branch naming

Always branch off `main`. Name branches using this pattern:

```
<type>/issue-<number>-<short-description>
```

Examples:
```
fix/issue-12-preview-pane-crash
feat/issue-34-marks-and-bookmarks
docs/issue-56-install-guide
```

If there is no issue, omit the issue segment:
```
fix/scroll-off-by-one
chore/update-dependencies
```

---

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

<body — explain WHY>

Closes #<issue-number>

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Scopes:** `nav`, `preview`, `search`, `icons`, `mouse`, `shell`, `ci`, `release`

**Issue linkage keywords** (GitHub closes the issue automatically on merge):
- `Closes #123` — use for features and bugs resolved by this change
- `Fixes #123` — alias for bugs specifically
- `Refs #123` — use when related but not fully resolved

**Example commit:**
```
fix(preview): prevent crash on empty binary files

Binary files with zero bytes caused an index-out-of-bounds panic
in the preview renderer. Skip rendering and show a placeholder instead.

Fixes #42

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
```

---

## Release process

Releases are triggered by pushing a version tag. Do this after merging all changes for a release:

```sh
# 1. Ensure Cargo.toml version matches the tag you're about to push
# 2. Ensure CHANGELOG.md has a dated section for this version
git tag v0.3.0
git push origin v0.3.0
```

The release workflow will:
1. Build binaries for `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. Create a GitHub Release with auto-generated notes
3. Update the Homebrew formula in `bradleyfay/homebrew-trek`

---

## Checklist before opening a PR

- [ ] `Cargo.toml` version bumped
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Branch named `<type>/issue-<number>-<description>` (if issue exists)
- [ ] Commit message references the issue with `Closes #N` or `Fixes #N`
- [ ] `cargo fmt` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
