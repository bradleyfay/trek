mod app;
mod archive;
mod args;
mod bookmarks;
mod datetime;
mod events;
mod find;
mod git;
mod highlight;
mod icons;
mod ops;
mod search;
mod session;
mod shell;
mod theme;
mod trash;
mod ui;
mod watcher;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    let parsed = match args::parse_args(&raw_args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("Try 'trek --help' for more information.");
            std::process::exit(1);
        }
    };

    if parsed.show_help {
        args::print_help();
        return Ok(());
    }

    if parsed.show_version {
        println!("trek {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if parsed.install_shell {
        return shell::install_shell_integration();
    }

    // Validate the optional starting directory before entering the TUI.
    let start_dir = if let Some(dir) = parsed.start_dir {
        let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if !canonical.is_dir() {
            eprintln!("trek: '{}' is not a directory", dir.display());
            std::process::exit(1);
        }
        Some(canonical)
    } else {
        None
    };

    // Resolve the theme before entering raw mode so that a bad --theme value
    // can be reported cleanly without leaving the terminal in raw/alt-screen state.
    let theme = match parsed.theme.as_deref() {
        None => crate::theme::Theme::default(),
        Some(name) => match crate::theme::Theme::from_name(name) {
            Some(t) => t,
            None => {
                eprintln!("trek: unknown theme '{}'.", name);
                eprintln!("Available themes:");
                for t in crate::theme::Theme::names() {
                    eprintln!("  {}", t);
                }
                eprintln!("Try 'trek --help' for more information.");
                std::process::exit(1);
            }
        },
    };

    // Install a panic hook that restores terminal state before printing the
    // panic message.  Without this, a panic leaves the terminal in raw mode
    // with the alternate screen active, requiring a blind `reset` to recover.
    events::setup_panic_hook();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = events::run(&mut terminal, start_dir, theme);

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
