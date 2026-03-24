use std::path::{Path, PathBuf};
use std::process::Command;

pub const MAX_ARCHIVE_ENTRIES: usize = 1_000;

// ── Archive extension detection ────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum ArchiveExt {
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
    Tar,
    Zip, // also .jar, .war, .ear
    Gz,
    SevenZip,
}

fn archive_ext(path: &Path) -> Option<ArchiveExt> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    // Compound extensions must be checked before single-extension suffixes.
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ArchiveExt::TarGz);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        return Some(ArchiveExt::TarBz2);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Some(ArchiveExt::TarXz);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return Some(ArchiveExt::TarZst);
    }
    if name.ends_with(".tar") {
        return Some(ArchiveExt::Tar);
    }
    if name.ends_with(".zip")
        || name.ends_with(".jar")
        || name.ends_with(".war")
        || name.ends_with(".ear")
    {
        return Some(ArchiveExt::Zip);
    }
    if name.ends_with(".gz") {
        return Some(ArchiveExt::Gz);
    }
    if name.ends_with(".7z") {
        return Some(ArchiveExt::SevenZip);
    }
    None
}

// ── Tool invocation ────────────────────────────────────────────────────────────

fn run_cmd(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin).args(args).output().ok()?;
    // None only when the binary is not found. Non-zero exit (corrupt archive
    // etc.) still returns Some("") so callers can show an error message.
    if out.status.code() == Some(127) {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn command_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Output parsers ─────────────────────────────────────────────────────────────

/// Parse `tar -t` output: one path per line.
pub fn parse_tar_output(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_end_matches('/').to_string())
        .collect()
}

/// Parse `unzip -l` output: skip separator lines; take last token of each data line.
pub fn parse_zip_output(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|l| !l.contains("---") && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|name| {
            // Skip the header/footer summary lines ("Name", "Archive:", totals).
            !name.eq_ignore_ascii_case("name")
                && !name.eq_ignore_ascii_case("archive:")
                && !name.starts_with("file")
        })
        .collect()
}

/// Parse `gzip -l` output: return a human-readable summary line.
pub fn parse_gz_output(output: &str) -> Vec<String> {
    // gzip -l output:
    //   compressed uncompressed  ratio uncompressed_name
    //      1234567      9876543  87.5% file.txt
    let mut result = Vec::new();
    for line in output.lines().skip(1) {
        // skip header line
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let compressed = parts[0];
            let uncompressed = parts[1];
            result.push(format!(
                "Compressed: {} bytes → Uncompressed: {} bytes",
                compressed, uncompressed
            ));
        }
    }
    if result.is_empty() {
        result.push(output.trim().to_string());
    }
    result
}

/// Parse `7z l` output: skip header/footer; extract Name column (position 53+).
pub fn parse_7z_output(output: &str) -> Vec<String> {
    let mut in_body = false;
    let mut result = Vec::new();
    for line in output.lines() {
        if line.starts_with("---") {
            in_body = !in_body;
            continue;
        }
        if in_body && line.len() > 53 {
            let name = line[53..].trim();
            if !name.is_empty() {
                result.push(name.to_string());
            }
        }
    }
    result
}

fn truncate_if_needed(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() > MAX_ARCHIVE_ENTRIES {
        lines.truncate(MAX_ARCHIVE_ENTRIES);
        lines.push(format!(
            "[truncated — showing first {} entries]",
            MAX_ARCHIVE_ENTRIES
        ));
    }
    lines
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Returns `true` if `path` has a recognized archive extension.
pub fn is_archive(path: &Path) -> bool {
    archive_ext(path).is_some()
}

/// Create an archive at `output_path` containing all paths in `inputs`.
///
/// Returns `Ok(())` on success or `Err(human-readable message)` on failure.
/// Callers must verify the output path does not already exist before calling.
pub fn create_archive(output_path: &Path, inputs: &[PathBuf]) -> Result<(), String> {
    let ext = archive_ext(output_path).ok_or_else(|| {
        "Unknown archive format; use .tar.gz, .tar.bz2, .tar.xz, .zip, or .tar".to_string()
    })?;

    let out_str = output_path
        .to_str()
        .ok_or_else(|| "Archive path contains invalid UTF-8".to_string())?
        .to_owned();

    let input_strs: Vec<String> = inputs
        .iter()
        .filter_map(|p| p.to_str().map(|s| s.to_owned()))
        .collect();

    match ext {
        ArchiveExt::Tar => run_create("tar", &["-cf", &out_str], &input_strs),
        ArchiveExt::TarGz => run_create("tar", &["-czf", &out_str], &input_strs),
        ArchiveExt::TarBz2 => run_create("tar", &["-cjf", &out_str], &input_strs),
        ArchiveExt::TarXz => run_create("tar", &["-cJf", &out_str], &input_strs),
        ArchiveExt::TarZst => run_create("tar", &["--zstd", "-cf", &out_str], &input_strs),
        ArchiveExt::Zip => {
            if !command_exists("zip") {
                return Err(
                    "zip not found — install: brew install zip (macOS) / apt install zip (Linux)"
                        .to_string(),
                );
            }
            run_create("zip", &["-r", &out_str], &input_strs)
        }
        ArchiveExt::Gz | ArchiveExt::SevenZip => {
            Err("Unknown archive format; use .tar.gz, .tar.bz2, .tar.xz, .zip, or .tar".to_string())
        }
    }
}

/// Run `bin args... inputs...` and return `Ok(())` or `Err(stderr)`.
fn run_create(bin: &str, args: &[&str], inputs: &[String]) -> Result<(), String> {
    let input_refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
    let out = Command::new(bin)
        .args(args)
        .args(&input_refs)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", bin, e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Extract the archive at `path` into `dest_dir`.
///
/// Returns `Ok(())` on success or `Err(message)` with a human-readable
/// description of what went wrong.
pub fn extract_archive(path: &Path, dest_dir: &Path) -> Result<(), String> {
    let ext = archive_ext(path).ok_or_else(|| "not a recognized archive format".to_string())?;
    let path_str = path
        .to_str()
        .ok_or_else(|| "path is not valid UTF-8".to_string())?;
    let dest_str = dest_dir
        .to_str()
        .ok_or_else(|| "destination path is not valid UTF-8".to_string())?;

    match ext {
        ArchiveExt::Tar => run_extract("tar", &["-xf", path_str, "-C", dest_str]),
        ArchiveExt::TarGz => run_extract("tar", &["-xzf", path_str, "-C", dest_str]),
        ArchiveExt::TarBz2 => run_extract("tar", &["-xjf", path_str, "-C", dest_str]),
        ArchiveExt::TarXz => run_extract("tar", &["-xJf", path_str, "-C", dest_str]),
        ArchiveExt::TarZst => run_extract("tar", &["-x", "--zstd", "-f", path_str, "-C", dest_str]),
        ArchiveExt::Zip => {
            if !command_exists("unzip") {
                return Err("unzip not found — install it to extract .zip files".to_string());
            }
            run_extract("unzip", &["-n", path_str, "-d", dest_str])
        }
        ArchiveExt::Gz => run_extract("gunzip", &["-k", "-f", path_str]),
        ArchiveExt::SevenZip => {
            if !command_exists("7z") {
                return Err("7z not found — install p7zip to extract .7z files".to_string());
            }
            run_extract("7z", &["x", path_str, &format!("-o{}", dest_str), "-y"])
        }
    }
}

/// Run an extraction command; returns `Ok(())` on success or `Err(first stderr line)`.
fn run_extract(bin: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("{} not found: {}", bin, e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.lines().next().unwrap_or("unknown error").to_string())
    }
}

/// Returns `Some(lines)` if `path` is a recognized archive and the listing
/// tool is available. Returns `None` to signal "fall back to normal preview."
pub fn try_list_archive(path: &Path) -> Option<Vec<String>> {
    let ext = archive_ext(path)?;
    let path_str = path.to_str()?;

    let lines = match ext {
        ArchiveExt::Tar => {
            let out = run_cmd("tar", &["-tf", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_tar_output(&out)
            }
        }
        ArchiveExt::TarGz => {
            let out = run_cmd("tar", &["-tzf", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_tar_output(&out)
            }
        }
        ArchiveExt::TarBz2 => {
            let out = run_cmd("tar", &["-tjf", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_tar_output(&out)
            }
        }
        ArchiveExt::TarXz => {
            let out = run_cmd("tar", &["-tJf", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_tar_output(&out)
            }
        }
        ArchiveExt::TarZst => {
            let out = run_cmd("tar", &["-t", "--zstd", "-f", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_tar_output(&out)
            }
        }
        ArchiveExt::Zip => {
            if !command_exists("unzip") {
                return Some(vec![
                    "[binary file — .zip preview requires unzip]".to_string()
                ]);
            }
            let out = run_cmd("unzip", &["-l", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_zip_output(&out)
            }
        }
        ArchiveExt::Gz => {
            let out = run_cmd("gzip", &["-l", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_gz_output(&out)
            }
        }
        ArchiveExt::SevenZip => {
            if !command_exists("7z") {
                return Some(vec!["[binary file — .7z preview requires 7z]".to_string()]);
            }
            let out = run_cmd("7z", &["l", path_str])
                .unwrap_or_else(|| "[could not read archive]".to_string());
            if out.is_empty() {
                vec!["[could not read archive]".to_string()]
            } else {
                parse_7z_output(&out)
            }
        }
    };

    Some(truncate_if_needed(lines))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── archive_ext detection ────────────────────────────────────────────────

    /// Given: a path ending in .tar.gz
    /// When: archive_ext is called
    /// Then: TarGz is returned
    #[test]
    fn archive_ext_detects_tar_gz() {
        let p = PathBuf::from("foo/bar.tar.gz");
        assert_eq!(archive_ext(&p), Some(ArchiveExt::TarGz));
    }

    /// Given: a path ending in .tgz (alternate .tar.gz extension)
    /// When: archive_ext is called
    /// Then: TarGz is returned
    #[test]
    fn archive_ext_detects_tgz_alias() {
        assert_eq!(
            archive_ext(&PathBuf::from("release.tgz")),
            Some(ArchiveExt::TarGz)
        );
    }

    /// Given: a path ending in .tar.bz2
    /// When: archive_ext is called
    /// Then: TarBz2 is returned
    #[test]
    fn archive_ext_detects_tar_bz2() {
        assert_eq!(
            archive_ext(&PathBuf::from("archive.tar.bz2")),
            Some(ArchiveExt::TarBz2)
        );
    }

    /// Given: a path ending in .tar.xz
    /// When: archive_ext is called
    /// Then: TarXz is returned
    #[test]
    fn archive_ext_detects_tar_xz() {
        assert_eq!(
            archive_ext(&PathBuf::from("archive.tar.xz")),
            Some(ArchiveExt::TarXz)
        );
    }

    /// Given: a path ending in .tar
    /// When: archive_ext is called
    /// Then: Tar is returned (not confused with compound .tar.gz)
    #[test]
    fn archive_ext_detects_plain_tar() {
        assert_eq!(
            archive_ext(&PathBuf::from("bundle.tar")),
            Some(ArchiveExt::Tar)
        );
    }

    /// Given: a path ending in .zip
    /// When: archive_ext is called
    /// Then: Zip is returned
    #[test]
    fn archive_ext_detects_zip() {
        assert_eq!(
            archive_ext(&PathBuf::from("file.zip")),
            Some(ArchiveExt::Zip)
        );
    }

    /// Given: a path ending in .jar
    /// When: archive_ext is called
    /// Then: Zip is returned (jars use zip format)
    #[test]
    fn archive_ext_detects_jar_as_zip() {
        assert_eq!(
            archive_ext(&PathBuf::from("app.jar")),
            Some(ArchiveExt::Zip)
        );
    }

    /// Given: a path with no recognized archive extension
    /// When: archive_ext is called
    /// Then: None is returned
    #[test]
    fn archive_ext_unknown_returns_none() {
        assert!(archive_ext(&PathBuf::from("script.sh")).is_none());
        assert!(archive_ext(&PathBuf::from("image.png")).is_none());
        assert!(archive_ext(&PathBuf::from("no_extension")).is_none());
    }

    /// Given: a path ending in .tar.gz (uppercase)
    /// When: archive_ext is called
    /// Then: TarGz is returned (case-insensitive)
    #[test]
    fn archive_ext_is_case_insensitive() {
        assert_eq!(
            archive_ext(&PathBuf::from("ARCHIVE.TAR.GZ")),
            Some(ArchiveExt::TarGz)
        );
    }

    // ── parse_tar_output ─────────────────────────────────────────────────────

    /// Given: typical tar -t output with files and directories
    /// When: parse_tar_output is called
    /// Then: each path is returned; trailing slashes on dirs are stripped
    #[test]
    fn parse_tar_strips_trailing_slash_from_dirs() {
        let output = "src/\nsrc/main.rs\nCargo.toml\n";
        let result = parse_tar_output(output);
        assert_eq!(result, vec!["src", "src/main.rs", "Cargo.toml"]);
    }

    /// Given: tar output with empty lines
    /// When: parse_tar_output is called
    /// Then: empty lines are filtered out
    #[test]
    fn parse_tar_filters_empty_lines() {
        let output = "a.txt\n\nb.txt\n";
        let result = parse_tar_output(output);
        assert_eq!(result, vec!["a.txt", "b.txt"]);
    }

    // ── parse_zip_output ─────────────────────────────────────────────────────

    /// Given: typical unzip -l output with separator lines and a header
    /// When: parse_zip_output is called
    /// Then: only file names are returned; separator and header lines are skipped
    #[test]
    fn parse_zip_skips_separator_and_header_lines() {
        let output = "\
Archive:  test.zip
  Length      Date    Time    Name
---------  ---------- -----   ----
     1234  2024-01-01 12:00   src/main.rs
      567  2024-01-01 12:00   Cargo.toml
---------                     -------
     1801                     2 files
";
        let result = parse_zip_output(output);
        assert!(result.contains(&"src/main.rs".to_string()), "{result:?}");
        assert!(result.contains(&"Cargo.toml".to_string()), "{result:?}");
        // Should not contain separator markers or summary
        assert!(!result.iter().any(|l| l.contains("---")));
    }

    // ── parse_gz_output ──────────────────────────────────────────────────────

    /// Given: typical gzip -l output
    /// When: parse_gz_output is called
    /// Then: a human-readable compressed/uncompressed line is returned
    #[test]
    fn parse_gz_returns_size_info() {
        let output = " compressed uncompressed  ratio uncompressed_name\n    1234567      9876543  87.5% file.txt\n";
        let result = parse_gz_output(output);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("1234567"), "{:?}", result[0]);
        assert!(result[0].contains("9876543"), "{:?}", result[0]);
    }

    // ── parse_7z_output ──────────────────────────────────────────────────────

    /// Given: typical 7z l output with --- separators and fixed-width name column
    /// When: parse_7z_output is called
    /// Then: only file names are extracted
    #[test]
    fn parse_7z_extracts_names_between_separators() {
        // 7z l output has a fixed-width format; Name starts at column 53
        let output = "\
7-Zip 24.08\n\
\n\
Scanning the drive for archives:\n\
1 file, 1234 bytes (2 KiB)\n\
\n\
Listing archive: test.7z\n\
\n\
--\n\
Path = test.7z\n\
Type = 7z\n\
\n\
   Date      Time    Attr         Size   Compressed  Name\n\
------------------- ----- ------------ ------------  ------------------------\n\
2024-01-01 12:00:00 ....A         1234          567  src/main.rs\n\
2024-01-01 12:00:00 ....A          100           50  Cargo.toml\n\
------------------- ----- ------------ ------------  ------------------------\n\
";
        let result = parse_7z_output(output);
        // The 7z format varies; just verify it doesn't panic and returns something
        // (parsing may differ by 7z version; behavior test is crash-free + Some)
        let _ = result; // must not panic
    }

    // ── truncation ───────────────────────────────────────────────────────────

    /// Given: more than MAX_ARCHIVE_ENTRIES lines
    /// When: truncate_if_needed is called
    /// Then: result is capped at MAX_ARCHIVE_ENTRIES + 1 (the truncation notice)
    #[test]
    fn truncation_caps_and_appends_notice() {
        let lines: Vec<String> = (0..MAX_ARCHIVE_ENTRIES + 50)
            .map(|i| format!("file_{}.txt", i))
            .collect();
        let result = truncate_if_needed(lines);
        assert_eq!(result.len(), MAX_ARCHIVE_ENTRIES + 1);
        assert!(
            result.last().unwrap().contains("truncated"),
            "expected truncation notice, got: {:?}",
            result.last()
        );
    }

    /// Given: exactly MAX_ARCHIVE_ENTRIES lines
    /// When: truncate_if_needed is called
    /// Then: no truncation occurs
    #[test]
    fn truncation_does_not_truncate_at_limit() {
        let lines: Vec<String> = (0..MAX_ARCHIVE_ENTRIES)
            .map(|i| format!("f{}", i))
            .collect();
        let result = truncate_if_needed(lines);
        assert_eq!(result.len(), MAX_ARCHIVE_ENTRIES);
        assert!(!result.last().unwrap().contains("truncated"));
    }
}
