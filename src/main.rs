mod app;
mod git;
mod icons;
mod rename;
mod search;
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

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--install-shell") {
        return install_shell_integration();
    }

    let choosedir = args
        .iter()
        .position(|a| a == "--choosedir")
        .and_then(|i| args.get(i + 1).cloned());

    // Install a panic hook that restores terminal state before printing the
    // panic message.  Without this, a panic leaves the terminal in raw mode
    // with the alternate screen active, requiring a blind `reset` to recover.
    setup_panic_hook();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match result {
        Ok(final_dir) => {
            if let Some(ref path) = choosedir {
                // Write atomically: write to a temp file first, then rename.
                // A plain fs::write is not atomic — if the process is killed
                // mid-write the shell script reads a partial path and passes it
                // to `cd`, producing confusing behaviour.
                let tmp = format!("{}.tmp.{}", path, std::process::id());
                std::fs::write(&tmp, final_dir.to_string_lossy().as_bytes())?;
                std::fs::rename(&tmp, path)?;
            }
        }
        Err(e) => eprintln!("Error: {e:?}"),
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

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<std::path::PathBuf> {
    let mut app = App::new()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match event::read()? {
            Event::Key(key) => {
                // Clear status message on any keypress.
                app.clear_status();

                if app.show_help {
                    // Any key closes help overlay.
                    app.show_help = false;
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
                        KeyCode::Char('y') => app.yank_relative_path(),
                        KeyCode::Char('Y') => app.yank_absolute_path(),
                        KeyCode::Char('d') => app.toggle_diff_preview(),
                        KeyCode::Char('R') => app.refresh_git_status(),
                        KeyCode::Char('?') => app.show_help = true,
                        // Bulk rename
                        KeyCode::Char(' ') => app.toggle_selection(app.selected),
                        KeyCode::Char('v') => app.select_all(),
                        KeyCode::Char('r') => app.start_rename(),
                        KeyCode::Esc => app.clear_selections(),
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
    println!("    trek [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help             Print this help message");
    println!("        --install-shell    Install the `m` shell function into your shell rc file");
    println!("        --choosedir <path> Write the final directory to <path> on exit");
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
