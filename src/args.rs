use std::path::PathBuf;

/// Outcome of parsing the command-line arguments.
#[derive(Debug)]
pub struct ParsedArgs {
    pub show_help: bool,
    pub show_version: bool,
    pub install_shell: bool,
    /// Value of `--choosedir <path>` (internal shell-integration flag).
    pub choosedir: Option<String>,
    /// Optional starting directory (first non-flag positional argument).
    pub start_dir: Option<PathBuf>,
    /// Optional colour theme name (`--theme <name>`).
    pub theme: Option<String>,
}

/// Parse `args` (argv[1..]) into a `ParsedArgs`.
///
/// Returns `Err(message)` on unrecognized flags or missing `--choosedir` value.
pub fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        show_help: false,
        show_version: false,
        install_shell: false,
        choosedir: None,
        start_dir: None,
        theme: None,
    };

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--help" | "-h" => parsed.show_help = true,
            "--version" | "-V" => parsed.show_version = true,
            "--install-shell" => parsed.install_shell = true,
            "--choosedir" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "--choosedir requires a path argument".to_string())?;
                parsed.choosedir = Some(val.clone());
            }
            "--theme" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "--theme requires a name argument".to_string())?;
                parsed.theme = Some(val.clone());
            }
            a if a.starts_with('-') => {
                return Err(format!("trek: unrecognized option '{a}'"));
            }
            _ => {
                // Positional argument: first one wins as start_dir.
                if parsed.start_dir.is_none() {
                    parsed.start_dir = Some(PathBuf::from(arg));
                }
            }
        }
        i += 1;
    }

    Ok(parsed)
}

pub fn print_help() {
    println!("trek — a terminal file manager");
    println!();
    println!("USAGE:");
    println!("    trek [OPTIONS] [PATH]");
    println!();
    println!("ARGS:");
    println!("    [PATH]    Directory to open (defaults to current working directory)");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help             Print this help message");
    println!("    -V, --version          Print version information");
    println!("        --install-shell    Install the `m` shell function (enables cd-on-exit)");
    println!("        --theme <name>     Set the colour theme (see below)");
    println!();
    println!("KEYBINDINGS (inside the TUI):");
    println!("    j / Down    Move down          k / Up      Move up");
    println!("    l / Right   Enter directory    h / Left    Go to parent");
    println!("    g           Go to top          G           Go to bottom");
    println!("    ~           Go to home         .           Toggle hidden files");
    println!("    e           Jump to typed path (absolute, relative, or ~/…)");
    println!("    `<c>        Set mark 'c' — record current dir to letter slot (a-z A-Z)");
    println!("    '<c>        Jump to mark 'c' — navigate to the marked directory");
    println!("    [           Scroll preview up 5 lines  ]  Scroll preview down 5 lines");
    println!("    /           Fuzzy search       Ctrl+F      Content search (rg)");
    println!("    z           Frecency jump list (auto-ranked recent dirs)");
    println!("    y / Y       Yank relative / absolute path");
    println!(
        "    A           Yank path (pick format: r=relative  a=absolute  f=filename  p=parent dir)"
    );
    println!("    #           Toggle line numbers in preview pane");
    println!("    i           Toggle gitignore filter (hide .gitignored files)");
    println!("    d           Toggle diff preview R           Refresh git status");
    println!("    V           Toggle git log preview (file/dir commit history)");
    println!("    D           Toggle disk usage breakdown for selected directory");
    println!("    I           Watch mode — auto-refresh listing on filesystem changes");
    println!("    f           Compare two selected files (unified diff)");
    println!("    H           Toggle hash preview (SHA-256 checksum)");
    println!("    a           Toggle hex dump view (binary file inspection)");
    println!("    w           Toggle preview pane (hide/show right pane)");
    println!("    T           Toggle modification timestamps / file sizes in listing");
    println!("    U           Toggle preview word wrap (soft-wrap long lines)");
    println!("    N           Toggle directory item counts (vs raw block size)");
    println!("    J           Extend selection down  K           Extend selection up");
    println!("    Space       Toggle file selection v          Select all files");
    println!("    *           Select files by glob pattern (e.g. *.rs, *.log, test_?)");
    println!("    n / F2      Quick rename (inline bar pre-filled with current name)");
    println!("    r           Bulk rename selected files with regex  Esc  Clear selections");
    println!("    o           Open in $EDITOR        O           Open with system default");
    println!("    c           Copy current to clipboard C          Copy selected to clipboard");
    println!("    x           Cut current to clipboard  F           Inspect clipboard contents");
    println!("    p           Paste clipboard       Delete      Delete current file/dir");
    println!("    t           New empty file         M           Make new directory");
    println!("    L           Create symlink to selected entry (Unix only)");
    println!("    W           Duplicate selected entry in place (editable name bar)");
    println!("    Z           Extract archive to current directory");
    println!("    E           Create archive from selected files (tar.gz, zip, …)");
    println!("    X           Delete all selected");
    println!("    :           Open command palette");
    println!("    ?           Show help overlay  q           Quit");
    println!();
    println!("THEMES:");
    println!("    default              Dark — ANSI named colours (default)");
    println!("    catppuccin-mocha     Dark — Catppuccin Mocha");
    println!("    catppuccin-latte     Light — Catppuccin Latte");
    println!("    tokyo-night          Dark — Tokyo Night");
    println!("    tokyo-night-light    Light — Tokyo Night Light");
    println!();
    println!("    RGB themes (all except 'default') look best in a truecolor terminal.");
    println!("    Set COLORTERM=truecolor if colours appear incorrect.");
}

// ── Tests for parse_args ───────────────────────────────────────────────────────

#[cfg(test)]
mod cli_tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// Given: no arguments
    /// When: parse_args is called
    /// Then: all flags are false and start_dir is None
    #[test]
    fn no_args_is_default() {
        let p = parse_args(&s(&[])).unwrap();
        assert!(!p.show_help);
        assert!(!p.show_version);
        assert!(!p.install_shell);
        assert!(p.choosedir.is_none());
        assert!(p.start_dir.is_none());
    }

    /// Given: --help
    /// When: parse_args is called
    /// Then: show_help is true
    #[test]
    fn help_flag_long() {
        let p = parse_args(&s(&["--help"])).unwrap();
        assert!(p.show_help);
    }

    /// Given: -h
    /// When: parse_args is called
    /// Then: show_help is true
    #[test]
    fn help_flag_short() {
        let p = parse_args(&s(&["-h"])).unwrap();
        assert!(p.show_help);
    }

    /// Given: --version
    /// When: parse_args is called
    /// Then: show_version is true
    #[test]
    fn version_flag_long() {
        let p = parse_args(&s(&["--version"])).unwrap();
        assert!(p.show_version);
    }

    /// Given: -V
    /// When: parse_args is called
    /// Then: show_version is true
    #[test]
    fn version_flag_short() {
        let p = parse_args(&s(&["-V"])).unwrap();
        assert!(p.show_version);
    }

    /// Given: an unrecognized flag (e.g. --foo)
    /// When: parse_args is called
    /// Then: an Err is returned naming the unknown flag
    #[test]
    fn unknown_flag_returns_error() {
        let result = parse_args(&s(&["--foo"]));
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("--foo"), "error should name the flag: {msg}");
    }

    /// Given: a positional argument that is a valid directory path
    /// When: parse_args is called
    /// Then: start_dir is Some with that path
    #[test]
    fn positional_arg_sets_start_dir() {
        let tmp = std::env::temp_dir();
        let p = parse_args(&s(&[tmp.to_str().unwrap()])).unwrap();
        assert!(p.start_dir.is_some());
    }

    /// Given: --choosedir followed by a path
    /// When: parse_args is called
    /// Then: choosedir is Some with that path
    #[test]
    fn choosedir_flag_sets_value() {
        let p = parse_args(&s(&["--choosedir", "/tmp/out"])).unwrap();
        assert_eq!(p.choosedir.as_deref(), Some("/tmp/out"));
    }

    /// Given: --install-shell
    /// When: parse_args is called
    /// Then: install_shell is true
    #[test]
    fn install_shell_flag() {
        let p = parse_args(&s(&["--install-shell"])).unwrap();
        assert!(p.install_shell);
    }

    /// Given: --theme with a valid name
    /// When: parse_args is called
    /// Then: theme is Some with the given name
    #[test]
    fn theme_flag_sets_name() {
        let p = parse_args(&s(&["--theme", "catppuccin-mocha"])).unwrap();
        assert_eq!(p.theme.as_deref(), Some("catppuccin-mocha"));
    }

    /// Given: --theme with no following value
    /// When: parse_args is called
    /// Then: an Err is returned
    #[test]
    fn theme_flag_missing_value_returns_error() {
        let result = parse_args(&s(&["--theme"]));
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("--theme"), "error should name the flag: {msg}");
    }

    /// Given: no --theme flag
    /// When: parse_args is called
    /// Then: theme is None
    #[test]
    fn theme_absent_is_none() {
        let p = parse_args(&s(&[])).unwrap();
        assert!(p.theme.is_none());
    }

    /// Given: all five known theme names
    /// When: Theme::from_name is called for each
    /// Then: all return Some (none silently fall back to default)
    #[test]
    fn all_theme_names_resolve() {
        for name in crate::theme::Theme::names() {
            assert!(
                crate::theme::Theme::from_name(name).is_some(),
                "Theme::from_name returned None for registered name '{name}'"
            );
        }
    }

    /// Given: an unrecognised theme name
    /// When: Theme::from_name is called
    /// Then: returns None
    #[test]
    fn unknown_theme_name_returns_none() {
        assert!(crate::theme::Theme::from_name("nonsense-theme").is_none());
    }
}
