# Contributing to Trek

Trek welcomes contributions. The development workflow is strict but
straightforward — it exists to keep the codebase clean and releases reliable.

---

## Before You Start

Read the open issue you intend to work on carefully. Extract two things before
writing any code:

1. The desired behavior — what should Trek do that it doesn't do today, or what
   should it stop doing?
2. The acceptance criteria — how will you and reviewers know the work is
   complete?

If either of these is unclear, ask in the issue before opening a branch.

---

## Development Workflow

Every change follows this sequence, without exception:

1. **Read the issue.** Extract desired behavior and acceptance criteria.
2. **Write a failing test first.** Tests are written in BDD style:
   Given a starting state, When an action occurs, Then the outcome is X.
3. **Implement the minimum change** needed to satisfy the test.
4. **Make all tests pass.**
5. **Run the quality gates** (see below). All four must pass.
6. **Bump the version** in `Cargo.toml` and add a CHANGELOG entry under
   `[Unreleased]`.
7. **Open a PR** with a conventional commit message.

Do not skip steps. In particular, do not write the implementation before the
test — the test defines the contract that the implementation must satisfy.

---

## Quality Gates

Run these four commands before every commit. All four must pass with zero
warnings and zero errors:

```sh
cargo fmt
cargo clippy -- -D warnings
cargo build --release
cargo test
```

Warnings may not be suppressed with `#[allow(...)]` without a justification
comment explaining why the suppression is appropriate. Reviewers will ask about
any suppression that lacks an explanation.

---

## Commit Messages

Trek uses [Conventional Commits](https://www.conventionalcommits.org/). Every
commit message must follow this format:

```
<type>(<scope>): <description>

<body — explain WHY, not what>

Closes #<issue-number>

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
```

**Types:**

| Type | When to use |
|------|-------------|
| `feat` | New feature or new keybinding |
| `fix` | Bug fix |
| `docs` | Documentation changes only |
| `style` | Code formatting, no logic change |
| `refactor` | Internal restructuring, no behavior change |
| `test` | Adding or updating tests |
| `chore` | Maintenance, tooling, CI/CD |

**Scopes:**

| Scope | Area of the codebase |
|-------|----------------------|
| `nav` | Navigation and directory traversal |
| `preview` | Preview pane and preview modes |
| `search` | Fuzzy search and ripgrep content search |
| `icons` | File type icons |
| `mouse` | Mouse input handling |
| `shell` | Shell integration (`trek --install-shell`) |
| `ci` | CI/CD workflows |
| `release` | Version bumps, CHANGELOG, Homebrew formula |

The body should explain why the change is being made, not describe what the
diff contains. Reviewers can read the diff; they benefit from understanding your
reasoning.

---

## Branch Naming

Branches follow this pattern:

```
<type>/issue-<number>-<short-description>
```

Examples:

```
feat/issue-34-marks-bookmarks
fix/issue-12-preview-crash
refactor/issue-67-datetime-module
docs/issue-88-keybinding-reference
```

Use the issue number. If there is no issue for the work you want to do, open
one first.

---

## Version Bumping

Trek follows [Semantic Versioning](https://semver.org/). Choose the version
component to bump based on the nature of the change:

| Change type | Version component |
|-------------|------------------|
| Bug fix, refactor, documentation | PATCH (0.x.**y**) |
| New feature, new keybinding | MINOR (0.**x**.0) |
| Breaking change — different keybindings, removed flag, changed behavior | MAJOR (**x**.0.0) |

Update `Cargo.toml` and add an entry to `CHANGELOG.md` under the `[Unreleased]`
heading. Do not create a new heading — the release process handles that.

---

## Release Process

Do not push a release tag until CI is green for the commit you intend to tag.
Verify CI status first:

```sh
gh run list --limit 5
```

Once CI is green, tag and push:

```sh
git tag v0.x.y
git push origin v0.x.y
```

The release workflow handles the rest:

- Builds macOS binaries for `aarch64` and `x86_64`
- Uploads the binaries as GitHub release assets
- Updates the Homebrew formula in `bradleyfay/homebrew-trek` automatically

Do not manually edit the Homebrew formula or create the GitHub release
yourself. The workflow does both.

---

## Architecture Principles

Trek's codebase is organized around SOLID principles. Keep these in mind when
deciding where new code belongs and how to structure new types.

**Single Responsibility** — each module does one thing. A module that handles
directory listing does not also handle preview rendering.

**Open/Closed** — extend Trek's behavior by adding new types that implement
existing traits. Avoid patching existing types to handle new cases.

**Liskov Substitution** — if a type implements a Trek trait, it must be fully
substitutable for any other implementation of that trait. Partial
implementations that panic or no-op on certain inputs are not acceptable.

**Interface Segregation** — traits should be narrow and focused. A type that
only needs to read files should not be required to implement a trait that also
writes files.

**Dependency Inversion** — depend on abstractions, not concrete I/O. Code that
operates on files should accept a trait object or generic parameter, not a
direct reference to `std::fs`. This is what makes the codebase testable without
a real filesystem.

**Cohesion** — modules that change together stay together. If two pieces of
code are always modified in the same PR, they probably belong in the same
module.

---

## Getting Help

If you are unsure whether a contribution fits Trek's scope, open a discussion
on the issue before starting work. The vision document at
[`CLAUDE.md`](../../CLAUDE.md) defines what Trek is and is not — checking a
proposed feature against that document before implementing it will save
everyone time.

---

## See Also

- [Installation](../getting-started/installation.md)
- [Quick Start](../getting-started/quick-start.md)
- [Trek on GitHub](https://github.com/bradleyfay/trek)
