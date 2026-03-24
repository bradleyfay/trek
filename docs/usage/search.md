# Search

Trek provides two distinct search capabilities: fuzzy file name search within the current directory, and full-text content search across the whole project. A separate filter lets you hide gitignored files from the listing entirely.

---

## Fuzzy File Search (`/`)

Press `/` to enter fuzzy search mode. The center pane filters in real time as you type.

| Key | Action |
|-----|--------|
| Type | Incrementally filter files in the current directory |
| `Tab` / `↓` | Move to the next match |
| `Shift+Tab` / `↑` | Move to the previous match |
| `Enter` | Confirm selection and exit search mode |
| `Esc` | Cancel and restore the full listing |

Fuzzy search operates on the current directory only. It does not recurse into subdirectories.

---

## Content Search (`Ctrl+F`)

Press `Ctrl+F` to open content search. Trek uses ripgrep to search file contents across the current project from the working directory down.

- Results are shown in the center pane
- Navigate results with `j` / `k`
- Press `Enter` on a result to jump to the file that contains the match

Content search respects `.gitignore` by default, consistent with ripgrep's standard behavior.

---

## Gitignore Filter (`i`)

Press `i` to hide gitignored files from the directory listing.

- Trek calls `git ls-files --others --ignored --exclude-standard` to determine which entries to suppress
- A yellow `[ignore]` badge appears in the path bar while the filter is active
- The filter is re-applied automatically on every directory load
- Press `i` again to disable the filter
- If the current directory is not inside a git repository, Trek shows an error and does not enable the filter

The gitignore filter works independently of fuzzy search and content search — it can be active at the same time as either.
