# Git Integration

Trek surfaces git information inline in the file listing and preview pane. It does not perform git operations — commits, branching, and merging belong in your git client. Trek's role is to show you the state of the repository at a glance and let you inspect diffs and history without leaving the file browser.

---

## Status Overlays

When Trek detects a git repository, each entry in the listing shows a colored status indicator alongside its name:

| Indicator | Color | Meaning |
|-----------|-------|---------|
| `●` | Yellow | Modified — tracked file with unstaged changes |
| `✚` | Green | Staged — changes added to the index (also shown for files with both staged and unstaged changes) |
| `+` | Cyan | Untracked — new file not yet known to git |
| `✖` | Red | Deleted — tracked file removed from disk |
| `✖` | Red | Conflict — file in an unmerged state |

The path bar also shows the current branch name so you always know which branch you are on.

---

## Diff Preview (`d`)

Press `d` to switch the preview pane to git diff mode.

- Shows the output of `git diff HEAD -- <file>` for the selected file
- Preview title shows `[diff]`
- Scrollable with `[` / `]` and the mouse scroll wheel
- Press `d` again to return to the default preview

---

## Git Log Preview (`V`)

Press `V` to switch the preview pane to git log mode.

- Shows the output of `git log --oneline -30 -- <path>`
- Works for both files and directories. For directories, the log includes commits that touched any file in the subtree.
- Preview title shows `[log]`
- Scrollable with `[` / `]` and the mouse scroll wheel
- Shows a clear error message if the current directory is not a git repository or has no commits
- Press `V` again to return to the default preview

---

## Gitignore Filter (`i`)

Press `i` to hide gitignored files from the listing. A yellow `[ignore]` badge appears in the path bar while the filter is active.

See [Search — Gitignore Filter](search.md#gitignore-filter-i) for full details.

---

## What Trek Does Not Do

Trek intentionally excludes git write operations. It will not stage, commit, checkout, merge, rebase, or push. If you reach for one of those actions while browsing, open your terminal or a dedicated git tool. Trek's job is navigation and inspection, not repository management.
