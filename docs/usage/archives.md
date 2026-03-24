# Archives

Trek can browse into archives as virtual directories and extract them directly from the file listing.

---

## Browsing Archives

Navigating into a recognized archive file (e.g. `project.tar.gz`) opens it as a virtual directory. The three-pane view works normally — you can navigate the archive's contents, preview files inside it, and inspect the structure without extracting anything first.

The path bar reflects the virtual path (e.g. `project/archive.tar.gz/src/`). Archives are read-only in browse mode.

---

## Extracting Archives (`Z`)

Press `Z` on any recognized archive file to extract it into the current directory.

A confirmation bar appears before extraction proceeds:

```
[ Extract ] "filename" → ./ [y/Enter · Esc to cancel]
```

Press `y` or `Enter` to confirm, or `Esc` to cancel.

**Supported formats:**

| Extension | Format |
|-----------|--------|
| `.tar` | Uncompressed tar |
| `.tar.gz`, `.tgz` | Gzip-compressed tar |
| `.tar.bz2`, `.tbz2` | Bzip2-compressed tar |
| `.tar.xz`, `.txz` | XZ-compressed tar |
| `.tar.zst`, `.tzst` | Zstandard-compressed tar |
| `.zip`, `.jar`, `.war`, `.ear` | ZIP |
| `.gz` | Gzip |
| `.7z` | 7-Zip |

**Outcome messages:**

- On success — the listing refreshes and the status bar shows `"Extracted: <name>"`
- On failure — the status bar shows `"Extract failed: <first stderr line>"`
- Non-archive selected — the status bar shows `"Not an archive"`
- Missing tool — Trek shows a helpful error if `unzip` or `7z` are not installed

---

## Summary

| Key | Operation |
|-----|-----------|
| `Z` | Extract the selected archive into the current directory |
