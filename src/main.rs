mod app;
mod icons;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<()> {
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

    if let Err(e) = result {
        eprintln!("Error: {e:?}");
    }
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match event::read()? {
            Event::Key(key) => {
                // Clear status message on any keypress.
                app.status_message = None;

                if app.show_help {
                    // Any key closes help overlay.
                    app.show_help = false;
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
                        KeyCode::Char('Q') => break,
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
                        KeyCode::Char('?') => app.show_help = true,
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
    Ok(())
}
