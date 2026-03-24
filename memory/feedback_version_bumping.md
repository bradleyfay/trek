---
name: Always bump version on changes
description: Every code change to trek must include a Cargo.toml version bump and CHANGELOG entry
type: feedback
---

Always bump the version in Cargo.toml and update CHANGELOG.md when making any change to the trek project.

**Why:** The release pipeline is tag-based — version numbers are the source of truth. Skipping a bump means changes get silently bundled into the next release with no clear record, and the CHANGELOG becomes inaccurate. The user explicitly called this out as a repeated bad habit.

**How to apply:** Before committing any change to trek (code, docs, config):
1. Decide patch/minor/major based on semver rules
2. Bump the version in `Cargo.toml`
3. Add an entry under `## [Unreleased]` in `CHANGELOG.md`
4. Include both files in the same commit as the change
