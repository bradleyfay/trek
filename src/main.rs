mod app;
mod archive;
mod bookmarks;
mod find;
mod git;
mod highlight;
mod icons;
mod ops;
mod rename;
mod search;
mod trash;
mod ui;

use anyhow::{bail, Result};
use app::App;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;

// ── Argument parsing ───────────────────────────────────────────────────────────

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
}

fn main() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    let parsed = match parse_args(&raw_args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("Try 'trek --help' for more information.");
            std::process::exit(1);
        }
    };

    if parsed.show_help {
        print_help();
        return Ok(());
    }

    if parsed.show_version {
        println!("trek {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if parsed.install_shell {
        return install_shell_integration();
    }

    // Validate the optional starting directory before entering the TUI.
    let start_dir: Option<PathBuf> = if let Some(dir) = parsed.start_dir {
        let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if !canonical.is_dir() {
            eprintln!("trek: '{}' is not a directory", dir.display());
            std::process::exit(1);
        }
        Some(canonical)
    } else {
        None
    };

    // Install a panic hook that restores terminal state before printing the
    // panic message.  Without this, a panic leaves the terminal in raw mode
    // with the alternate screen active, requiring a blind `reset` to recover.
    setup_panic_hook();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, start_dir);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match result {
        Ok(final_dir) => {
            if let Some(ref path) = parsed.choosedir {
                // Write atomically: write to a temp file first, then rename.
                // A plain fs::write is not atomic — if the process is killed
                // mid-write the shell script reads a partial path and passes it
                // to `cd`, producing confusing behaviour.
                let tmp = format!("{}.tmp.{}", path, std::process::id());
                std::fs::write(&tmp, final_dir.to_string_lossy().as_bytes())?;
                std::fs::rename(&tmp, path)?;
            }
        }
        Err(e) => {
            // Issue #9: print error to stderr and exit 1, not 0.
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Install a panic hook that restores the terminal before the default hook
/// prints the panic message.  This ensures the terminal is usable after a
/// crash without needing to run `reset`.
fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort — ignore errors; we're already panicking.
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: Option<PathBuf>,
) -> Result<PathBuf> {
    let mut app = App::new(start_dir)?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match event::read()? {
            Event::Key(key) => {
                // Clear status message on any keypress.
                app.clear_status();

                if app.show_help {
                    // Any key closes help overlay.
                    app.show_help = false;
                } else if !app.pending_delete.is_empty() {
                    // t/y → trash (recoverable); D → permanent delete; anything else → cancel.
                    match key.code {
                        KeyCode::Char('t') | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_trash()
                        }
                        KeyCode::Char('D') => app.confirm_permanent_delete(),
                        _ => app.cancel_delete(),
                    }
                } else if app.mkdir_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_mkdir(),
                        KeyCode::Enter => app.confirm_mkdir(),
                        KeyCode::Backspace => app.mkdir_pop_char(),
                        KeyCode::Char(c) => app.mkdir_push_char(c),
                        _ => {}
                    }
                } else if app.content_search_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_content_search(),
                        KeyCode::Enter => app.run_content_search(),
                        KeyCode::Backspace => app.content_search_pop_char(),
                        KeyCode::Up | KeyCode::Char('k') => app.content_search_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.content_search_move_down(),
                        KeyCode::Char('l') | KeyCode::Right => app.jump_to_content_result(),
                        KeyCode::Char(c) => app.content_search_push_char(c),
                        _ => {}
                    }
                } else if app.rename_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_rename(),
                        KeyCode::Enter => app.confirm_rename(),
                        KeyCode::Tab => app.rename_next_field(),
                        KeyCode::BackTab => app.rename_prev_field(),
                        KeyCode::Backspace => app.rename_pop_char(),
                        KeyCode::Char(c) => app.rename_push_char(c),
                        _ => {}
                    }
                } else if app.bookmark_mode {
                    match key.code {
                        KeyCode::Esc => app.close_bookmarks(),
                        KeyCode::Char('B') => app.close_bookmarks(),
                        KeyCode::Enter => app.confirm_bookmark(),
                        KeyCode::Char('d') => app.remove_bookmark(),
                        KeyCode::Up | KeyCode::Char('k') => app.bookmark_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.bookmark_move_down(),
                        KeyCode::Backspace => app.bookmark_pop_char(),
                        KeyCode::Char(c) => app.bookmark_push_char(c),
                        _ => {}
                    }
                } else if app.find_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_find(),
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.cancel_find()
                        }
                        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                            app.jump_to_find_result()
                        }
                        KeyCode::Backspace => app.find_pop_char(),
                        KeyCode::Up | KeyCode::Char('k') => app.find_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.find_move_down(),
                        KeyCode::Char(c) => app.find_push_char(c),
                        _ => {}
                    }
                } else if app.chmod_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_chmod(),
                        KeyCode::Enter => app.confirm_chmod(),
                        KeyCode::Backspace => app.chmod_pop_char(),
                        KeyCode::Char(c @ '0'..='7') => app.chmod_push_char(c),
                        _ => {}
                    }
                } else if app.filter_mode {
                    match key.code {
                        KeyCode::Esc => app.clear_filter(),
                        KeyCode::Enter => app.close_filter(),
                        KeyCode::Char('|') => app.close_filter(),
                        KeyCode::Backspace => app.filter_pop_char(),
                        KeyCode::Char(c) => app.filter_push_char(c),
                        _ => {}
                    }
                } else if app.search_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_search(),
                        KeyCode::Enter => app.confirm_search(),
                        KeyCode::Backspace => app.search_pop_char(),
                        KeyCode::Up | KeyCode::BackTab => app.search_move_up(),
                        KeyCode::Down | KeyCode::Tab => app.search_move_down(),
                        KeyCode::Char(c) => app.search_push_char(c),
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.start_content_search()
                        }
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.start_find()
                        }
                        KeyCode::Char('Q') | KeyCode::Char('q') => break,
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Left | KeyCode::Char('h') => app.go_parent(),
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                            app.enter_selected()
                        }
                        KeyCode::Char('g') => app.go_top(),
                        KeyCode::Char('G') => app.go_bottom(),
                        KeyCode::Char('~') => app.go_home(),
                        KeyCode::Char('.') => app.toggle_hidden(),
                        KeyCode::Char('/') => app.start_search(),
                        KeyCode::Char('|') => app.start_filter(),
                        KeyCode::Char('y') => app.yank_relative_path(),
                        KeyCode::Char('Y') => app.yank_absolute_path(),
                        KeyCode::Char('d') => app.toggle_diff_preview(),
                        KeyCode::Char('m') => app.toggle_meta_preview(),
                        KeyCode::Char('P') => app.begin_chmod(),
                        KeyCode::Char('R') => app.refresh_git_status(),
                        KeyCode::Char('?') => app.show_help = true,
                        // Bulk rename
                        KeyCode::Char(' ') => app.toggle_selection(app.selected),
                        KeyCode::Char('v') => app.select_all(),
                        KeyCode::Char('r') => app.start_rename(),
                        KeyCode::Esc => {
                            if !app.filter_input.is_empty() {
                                app.clear_filter();
                            } else {
                                app.clear_selections();
                            }
                        }
                        // File operations
                        KeyCode::Char('c') => app.clipboard_copy_current(),
                        KeyCode::Char('C') => app.clipboard_copy_selected(),
                        KeyCode::Char('x') => app.clipboard_cut_current(),
                        KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.paste_clipboard()
                        }
                        KeyCode::Delete => app.begin_delete_current(),
                        KeyCode::Char('X') => app.begin_delete_selected(),
                        KeyCode::Char('M') => app.begin_mkdir(),
                        KeyCode::Char('u') => app.undo_trash(),
                        KeyCode::Char('b') => app.add_bookmark(),
                        KeyCode::Char('B') => app.open_bookmarks(),
                        KeyCode::Char('S') => app.cycle_sort_mode(),
                        KeyCode::Char('s') => app.toggle_sort_order(),
                        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.history_back()
                        }
                        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.history_forward()
                        }
                        _ => {}
                    }
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if app.show_help {
                        app.show_help = false;
                    } else {
                        app.on_mouse_down(mouse.column, mouse.row);
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    app.on_mouse_drag(mouse.column, mouse.row);
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    app.on_mouse_up();
                }
                MouseEventKind::ScrollUp => {
                    app.on_scroll_up(mouse.column, mouse.row);
                }
                MouseEventKind::ScrollDown => {
                    app.on_scroll_down(mouse.column, mouse.row);
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
    Ok(app.cwd)
}

fn print_help() {
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
    println!();
    println!("KEYBINDINGS (inside the TUI):");
    println!("    j / Down    Move down          k / Up      Move up");
    println!("    l / Right   Enter directory    h / Left    Go to parent");
    println!("    g           Go to top          G           Go to bottom");
    println!("    ~           Go to home         .           Toggle hidden files");
    println!("    /           Fuzzy search       Ctrl+F      Content search (rg)");
    println!("    y / Y       Yank relative / absolute path");
    println!("    d           Toggle diff preview R           Refresh git status");
    println!("    Space       Toggle file selection v          Select all files");
    println!("    r           Rename selected files Esc        Clear selections");
    println!("    c           Copy current to clipboard C          Copy selected to clipboard");
    println!("    x           Cut current to clipboard");
    println!("    p           Paste clipboard       Delete      Delete current file/dir");
    println!("    X           Delete all selected   M           Make new directory");
    println!("    ?           Show help overlay  q           Quit");
}

fn install_shell_integration() -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();

    // Validate HOME before constructing any paths from it.
    if home.is_empty() {
        bail!("HOME environment variable is not set; cannot determine shell profile path");
    }
    let home_path = std::path::Path::new(&home);
    if !home_path.is_absolute() {
        bail!(
            "HOME is not an absolute path ({:?}); refusing to write shell profile",
            home
        );
    }

    let (profile, snippet) = if shell.contains("zsh") {
        (
            format!("{home}/.zshrc"),
            r#"
# trek shell integration — added by `trek --install-shell`
m() {
  local tmp=$(mktemp)
  trek --choosedir "$tmp"
  local dir=$(cat "$tmp")
  rm -f "$tmp"
  [[ -n "$dir" ]] && cd "$dir"
}
"#,
        )
    } else if shell.contains("bash") {
        (
            format!("{home}/.bashrc"),
            r#"
# trek shell integration — added by `trek --install-shell`
m() {
  local tmp=$(mktemp)
  trek --choosedir "$tmp"
  local dir=$(cat "$tmp")
  rm -f "$tmp"
  [ -n "$dir" ] && cd "$dir"
}
"#,
        )
    } else if shell.contains("fish") {
        (
            format!("{home}/.config/fish/config.fish"),
            r#"
# trek shell integration — added by `trek --install-shell`
function m
  set tmp (mktemp)
  trek --choosedir $tmp
  set dir (cat $tmp)
  rm -f $tmp
  if test -n "$dir"
    cd $dir
  end
end
"#,
        )
    } else {
        bail!(
            "Unsupported shell: {:?}. Add the wrapper manually — see `trek --help`.",
            shell
        );
    };

    let profile_path = std::path::Path::new(&profile);
    let contents = std::fs::read_to_string(profile_path).unwrap_or_default();

    if contents.contains("trek --choosedir") {
        println!("Already installed in {profile}. Nothing to do.");
        return Ok(());
    }

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(profile_path)?;
    std::io::Write::write_all(&mut file, snippet.as_bytes())?;

    println!("Installed! Added `m` function to {profile}");
    println!("Reload your shell:  source {profile}");
    Ok(())
}
