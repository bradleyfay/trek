# Agent Guidelines for trek

This document describes how AI agents should contribute to trek. Follow these rules on every change — no exceptions.

---

## Development Philosophy

**1. Tests first**
Write a failing test before writing implementation code. No feature or fix ships without passing tests. Prefer behavior-level tests (Given/When/Then) over unit tests that only cover internals.

**2. Refactor aggressively**
Every change is an opportunity to improve the code it touches. If you encounter a module doing two things, split it. If you encounter duplicated logic, extract it. If a function is too long to test in isolation, break it up. Do not leave code worse than you found it. Do not leave code the same if it can be made clearer, faster, or better structured. This is not optional — refactoring is part of the work, not a separate task.

**3. Module boundaries are strict**
Each module owns one responsibility. State lives in `app/mod.rs`. Rendering lives in `ui.rs`. Preview logic lives in its own module. Cross-module field access is a design smell — expose only what callers need through well-defined interfaces. When a new feature requires new state or behavior, decide upfront whether it belongs in an existing module or warrants a new one.

---

## Feature workflow: from issue to release

Every feature or fix should flow through this process. For agents handling GitHub issues, this is the canonical path.

### Step 1 — Read and clarify the issue

Before writing any code, read the issue completely. Extract:
- The desired **behavior** (not just the implementation)
- Acceptance criteria or expected outcomes
- Edge cases or constraints mentioned

If acceptance criteria are missing, infer them from the issue context and state them explicitly in your PR description.

### Step 2 — Write a failing test first

Translate the acceptance criteria into one or more tests before implementing anything.

For behavior-driven scenarios, follow the **Given / When / Then** structure in test names and doc comments:

```rust
/// Given: a directory containing only binary files
/// When: the user navigates to a binary file
/// Then: the preview pane shows a placeholder, not a panic
#[test]
fn preview_binary_file_shows_placeholder() {
    // ...
}
```

For unit-level logic, standard Rust `#[test]` naming is fine — just be descriptive.

Run `cargo test` to confirm the test **fails** before writing the implementation. A test that passes before any code change is not testing the right thing.

### Step 3 — Implement the minimum required change

Write only what's needed to make the failing tests pass. Do not add speculative functionality or polish unrelated code.

Apply SOLID principles during implementation (see [Architecture rules](#architecture-rules-solid) below).

### Step 4 — Make all tests pass

Run `cargo test`. All tests — existing and new — must pass. Fix any regressions before continuing.

### Step 5 — Verify quality gates locally before every commit

Run these **before `git commit`**, not just before opening a PR. Pushing broken code to `main` fails CI and can cancel a release build mid-run.

```sh
cargo fmt
cargo clippy -- -D warnings
cargo build --release
cargo test
```

All four must pass clean. `cargo fmt` must be run (not just checked) so the formatting is actually applied. No warnings suppressed with `#[allow(...)]` without a comment explaining why — the one standing exception is `apply_layout` which is pre-existing and structurally fixed.

### Step 6 — Bump version and update CHANGELOG

See [Version bumping](#version-bumping-semantic-versioning) and [CHANGELOG format](#changelog-format) below. These happen in the same commit as the implementation.

### Step 7 — Open a PR

The PR triggers CI. When CI is green and the PR is merged to `main`, the feature is complete.

### Step 8 — Release (only after CI is green)

**Do not push a release tag until the CI run for the commit you intend to tag has passed.** Tagging before CI is green cancels the release build mid-run, because the release job depends on a clean build environment. A cancelled release leaves no GitHub Release and no Homebrew update.

Check CI status first:

```sh
gh run list --limit 5
```

Once the `Check` job shows `completed / success`, push the tag:

```sh
git tag v0.4.0
git push origin v0.4.0
```

The release workflow builds macOS binaries (`aarch64` and `x86_64` via cross-compilation from `macos-14`), creates a GitHub Release, and updates the Homebrew formula automatically.

If a release tag was pushed prematurely and the release was cancelled, delete the tag, fix the issue, re-run the quality gates, commit, wait for CI, then retag:

```sh
git tag -d v0.4.0
git push origin :v0.4.0
# fix, commit, wait for CI green
git tag v0.4.0
git push origin v0.4.0
```

---

## Architecture rules (SOLID)

These rules apply whenever you add or modify code. If a change would violate one of these, redesign before implementing.

### Single Responsibility

Each module, struct, and function does one thing. `App` manages navigation state. `ui.rs` renders state to the terminal. Preview logic belongs in a dedicated module — not embedded in `App` or the renderer.

When a new feature requires new state or new behavior, ask whether it belongs in an existing module or warrants a new one. A module that starts doing two unrelated things should be split.

### Open / Closed

Extend through new types and new modules, not by patching existing ones. For example, adding a new preview renderer for a file type means adding a new implementation, not adding another `if` branch to the existing renderer.

### Liskov Substitution

If a trait is defined (e.g., a `Previewer` trait), any type implementing it must be substitutable without breaking callers. Don't add escape hatches or special-case logic that defeats the abstraction.

### Interface Segregation

Traits should be narrow. A struct that only needs to read the current selection should not depend on a trait that also exposes write operations. Split broad interfaces into focused ones.

### Dependency Inversion

High-level modules (the app loop, rendering) should depend on abstractions, not on concrete filesystem or I/O calls. This makes behavior testable without touching the real filesystem. Use dependency injection via constructor parameters or trait objects where it makes the code testable.

### Cohesion and coupling

- Modules that change together should live together. Modules that don't should not be entangled.
- Avoid reaching across module boundaries to access internal fields. Expose only what's necessary.
- If two modules are becoming tightly coupled, introduce a shared abstraction or consolidate them.

---

## Every change requires three things

1. **A version bump** in `Cargo.toml`
2. **A CHANGELOG entry** in `CHANGELOG.md` — add under `## [Unreleased]` before committing, never after
3. **A conventional commit message** with issue linkage when applicable

These must be in the same commit as the change itself. Never skip them.

The CHANGELOG is the most commonly skipped step. Before running `git commit`, open `CHANGELOG.md` and confirm the `[Unreleased]` section describes what you just built. If it does not, write the entry first.

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

## Checklist before every commit

Run this before `git commit`. These gates must pass locally — do not rely on CI to catch them.

- [ ] **`CHANGELOG.md` updated under `[Unreleased]`** — do this first, before writing the commit message
- [ ] **`Cargo.toml` version bumped**
- [ ] `cargo fmt` run (not just checked — actually applied)
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes (no regressions, no skipped tests)
- [ ] Tests written before or alongside the implementation (failing first)
- [ ] All tests describe behavior using Given/When/Then where appropriate
- [ ] New code follows SOLID principles — no god structs, no cross-module coupling
- [ ] Commit message references the issue with `Closes #N` or `Fixes #N`

## Checklist before pushing a release tag

- [ ] All items above are done
- [ ] CI run for this commit shows `completed / success` (`gh run list --limit 3`)
- [ ] `Cargo.toml` version matches the tag you are about to push
- [ ] `CHANGELOG.md` `[Unreleased]` section renamed to `[x.y.z] - YYYY-MM-DD`
