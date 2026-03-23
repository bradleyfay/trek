mod app;
mod icons;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind},
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
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Left | KeyCode::Char('h') => app.go_parent(),
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => app.enter_selected(),
                KeyCode::Char('g') => app.go_top(),
                KeyCode::Char('G') => app.go_bottom(),
                KeyCode::Char('~') => app.go_home(),
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    app.on_mouse_down(mouse.column, mouse.row);
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
