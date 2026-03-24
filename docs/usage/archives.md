# Archives

Trek can extract and create archives directly from the file listing. The two operations form a symmetric pair: `Z` extracts, `E` creates.

---

## Extracting Archives (`Z`)

Press `Z` on any recognized archive file to extract it into the current directory.

A confirmation bar appears before extraction proceeds:

```
[ Extract ] "filename" → ./ [y/Enter · Esc to cancel]
```

Press `y` or `Enter` to confirm, or `Esc` to cancel.

**Supported formats for extraction:**

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

## Creating Archives (`E`)

Press `E` to open the archive creation input bar. The bar is pre-filled with a suggested name based on the current selection:

| Selection state | Pre-filled name |
|----------------|----------------|
| No selection | `<current entry name>.tar.gz` |
| 1 item selected | `<selected name>.tar.gz` |
| Multiple items selected | `archive.tar.gz` |

Type the desired archive name and press `Enter`. Trek infers the format from the file extension you provide.

**Supported formats for creation:**

| Extension | Format |
|-----------|--------|
| `.tar.gz`, `.tgz` | Gzip-compressed tar |
| `.tar.bz2`, `.tbz2` | Bzip2-compressed tar |
| `.tar.xz`, `.txz` | XZ-compressed tar |
| `.tar.zst`, `.tzst` | Zstandard-compressed tar |
| `.tar` | Uncompressed tar |
| `.zip` | ZIP |

!!! note "Unsupported creation formats"
    `.gz` and `.7z` are not supported for creation and will show a clear error message if specified. Use `.tar.gz` for gzip-compressed output.

**Outcome:**

- The created archive appears in the listing immediately with the cursor moved to it
- Clear errors are shown for unknown extensions, output files that already exist, or missing tools (e.g. `zip` not found)

---

## Summary

| Key | Operation |
|-----|-----------|
| `Z` | Extract the selected archive into the current directory |
| `E` | Create a new archive from the current entry or selection |
