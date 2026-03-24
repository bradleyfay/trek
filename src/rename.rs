use crate::app::DirEntry;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

/// Which input field has focus in rename mode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenameField {
    Pattern,
    Replacement,
}

/// One row in the live rename preview.
#[derive(Clone)]
pub struct RenamePreviewRow {
    pub original: String,
    pub result: RenameResult,
}

/// Outcome for a single file in the rename preview.
#[derive(Clone)]
pub enum RenameResult {
    /// File will be renamed to this new name.
    Renamed(String),
    /// Pattern did not match this filename (or new name equals old name).
    NoMatch,
    /// New name would collide with an existing file or another entry in this batch.
    Conflict(String),
}

/// Compute a rename preview for the given selected entries.
///
/// Returns `(rows, error_message)`. `error_message` is `Some` when `pattern`
/// is not a valid regular expression.
pub fn compute_preview(
    selected: &[&DirEntry],
    all_entries: &[DirEntry],
    pattern: &str,
    replacement: &str,
) -> (Vec<RenamePreviewRow>, Option<String>) {
    if pattern.is_empty() {
        let rows = selected
            .iter()
            .map(|e| RenamePreviewRow {
                original: e.name.clone(),
                result: RenameResult::NoMatch,
            })
            .collect();
        return (rows, None);
    }

    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            let rows = selected
                .iter()
                .map(|e| RenamePreviewRow {
                    original: e.name.clone(),
                    result: RenameResult::NoMatch,
                })
                .collect();
            // Trim the verbose regex error to something readable.
            let msg = e.to_string();
            let short = msg.lines().next().unwrap_or(&msg).to_string();
            return (rows, Some(short));
        }
    };

    let today = today_date();
    let existing: HashSet<&str> = all_entries.iter().map(|e| e.name.as_str()).collect();

    // Pass 1 — compute proposed new names (counter increments per matching file).
    let mut proposals: Vec<Option<String>> = Vec::with_capacity(selected.len());
    let mut counter = 0usize;
    for entry in selected {
        if !re.is_match(&entry.name) {
            proposals.push(None);
            continue;
        }
        counter += 1;
        let captures = re.captures(&entry.name);
        let new_name =
            expand_template(&entry.name, replacement, captures.as_ref(), counter, &today);
        if new_name == entry.name {
            proposals.push(None); // same name → treated as no-match
        } else {
            proposals.push(Some(new_name));
        }
    }

    // Pass 2 — conflict detection against the post-rename namespace.
    //
    // Names vacated by the renames: original names of entries that will be renamed.
    let vacated: HashSet<&str> = selected
        .iter()
        .zip(proposals.iter())
        .filter_map(|(e, p)| p.as_ref().map(|_| e.name.as_str()))
        .collect();

    // Count how many entries in this batch would produce each proposed name.
    let mut batch_count: HashMap<&str, usize> = HashMap::new();
    for name in proposals.iter().flatten() {
        *batch_count.entry(name.as_str()).or_insert(0) += 1;
    }

    let mut rows = Vec::with_capacity(selected.len());
    for (entry, proposal) in selected.iter().zip(proposals.iter()) {
        match proposal {
            None => rows.push(RenamePreviewRow {
                original: entry.name.clone(),
                result: RenameResult::NoMatch,
            }),
            Some(new_name) => {
                // A name conflicts if:
                //   • it exists in the directory AND will NOT be vacated by another rename, OR
                //   • two entries in this batch would rename to the same name.
                let stays =
                    existing.contains(new_name.as_str()) && !vacated.contains(new_name.as_str());
                let dupe = batch_count.get(new_name.as_str()).copied().unwrap_or(0) > 1;
                if stays || dupe {
                    rows.push(RenamePreviewRow {
                        original: entry.name.clone(),
                        result: RenameResult::Conflict(new_name.clone()),
                    });
                } else {
                    rows.push(RenamePreviewRow {
                        original: entry.name.clone(),
                        result: RenameResult::Renamed(new_name.clone()),
                    });
                }
            }
        }
    }

    (rows, None)
}

/// Apply the renames described by `rows` inside `cwd`.
///
/// Skips `NoMatch` and `Conflict` entries. Returns the count of successfully
/// renamed files, or an `Err` with a descriptive message on first failure.
pub fn apply_renames(rows: &[RenamePreviewRow], cwd: &Path) -> Result<usize, String> {
    let mut count = 0;
    for row in rows {
        if let RenameResult::Renamed(ref new_name) = row.result {
            let src = cwd.join(&row.original);
            let dst = cwd.join(new_name);
            std::fs::rename(&src, &dst)
                .map_err(|e| format!("Cannot rename '{}' → '{}': {}", row.original, new_name, e))?;
            count += 1;
        }
    }
    Ok(count)
}

// ── Template expansion ────────────────────────────────────────────────────────

/// Expand a replacement template, resolving capture groups and named tokens.
///
/// Order of expansion:
/// 1. Regex capture references (`$1`, `$2`, `$name`) via `Captures::expand`.
/// 2. Named tokens: `{stem}`, `{ext}`, `{date}`, `{n}`, `{n:0N}`.
fn expand_template(
    filename: &str,
    template: &str,
    captures: Option<&regex::Captures<'_>>,
    n: usize,
    today: &str,
) -> String {
    // Step 1: expand $1 / $2 / $name capture references.
    let after_captures: String = if let Some(caps) = captures {
        let mut out = String::new();
        caps.expand(template, &mut out);
        out
    } else {
        template.to_string()
    };

    // Step 2: expand {token} placeholders.
    let (stem, ext) = split_stem_ext(filename);
    let mut result = String::new();
    let mut iter = after_captures.chars().peekable();

    while let Some(ch) = iter.next() {
        if ch != '{' {
            result.push(ch);
            continue;
        }
        // Collect everything until the matching '}'.
        let mut token = String::new();
        let mut closed = false;
        for inner in iter.by_ref() {
            if inner == '}' {
                closed = true;
                break;
            }
            token.push(inner);
        }
        if !closed {
            // Unclosed brace — emit literally.
            result.push('{');
            result.push_str(&token);
            continue;
        }
        match token.as_str() {
            "stem" => result.push_str(stem),
            "ext" => result.push_str(ext),
            "date" => result.push_str(today),
            "n" => result.push_str(&n.to_string()),
            t if t.starts_with("n:") => {
                if let Some(width) = parse_pad_width(&t[2..]) {
                    result.push_str(&format!("{:0>width$}", n, width = width));
                } else {
                    result.push('{');
                    result.push_str(t);
                    result.push('}');
                }
            }
            _ => {
                // Unknown token — emit literally.
                result.push('{');
                result.push_str(&token);
                result.push('}');
            }
        }
    }
    result
}

/// Split `"foo.bar.txt"` into `("foo.bar", "txt")`.
///
/// A leading dot (hidden files: `".hidden"`) is not treated as an extension
/// separator, so `".hidden"` → `(".hidden", "")`.
fn split_stem_ext(name: &str) -> (&str, &str) {
    // Search for the last `.` that is not at position 0.
    if let Some(pos) = name[1..].rfind('.').map(|p| p + 1) {
        (&name[..pos], &name[pos + 1..])
    } else {
        (name, "")
    }
}

/// Parse a zero-padding width from a format spec like `"02"` → `Some(2)`.
///
/// Supports both `"02"` (leading zero) and `"2"` (plain digit width).
fn parse_pad_width(s: &str) -> Option<usize> {
    let digits = s.trim_start_matches('0');
    if digits.is_empty() {
        // All zeros e.g. "00" → width 0 (unusual but valid).
        Some(0)
    } else {
        digits.parse::<usize>().ok()
    }
}

/// Return today's date as `YYYY-MM-DD` using the system `date` command.
fn today_date() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "0000-00-00".to_string())
}
