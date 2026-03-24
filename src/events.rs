use crate::app::App;
use anyhow::Result;
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

/// Install a panic hook that restores the terminal before the default hook
/// prints the panic message.  This ensures the terminal is usable after a
/// crash without needing to run `reset`.
pub fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort — ignore errors; we're already panicking.
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: Option<PathBuf>,
) -> Result<PathBuf> {
    let mut app = App::new(start_dir)?;

    loop {
        terminal.draw(|f| crate::ui::draw(f, &mut app))?;

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
                } else if app.quick_rename_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_quick_rename(),
                        KeyCode::Enter => app.confirm_quick_rename(),
                        KeyCode::Backspace => app.quick_rename_pop_char(),
                        KeyCode::Char(c) => app.quick_rename_push_char(c),
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
                } else if app.path_mode {
                    match key.code {
                        KeyCode::Esc => app.cancel_path_jump(),
                        KeyCode::Enter => app.confirm_path_jump(),
                        KeyCode::Backspace => app.path_pop_char(),
                        KeyCode::Char(c) => app.path_push_char(c),
                        _ => {}
                    }
                } else if app.palette_mode {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char(':') => app.close_palette(),
                        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                            if let Some(action) = app.palette_selected_action() {
                                app.close_palette();
                                execute_palette_action(&mut app, action, terminal)?;
                            }
                        }
                        KeyCode::Backspace => app.palette_pop_char(),
                        KeyCode::Up | KeyCode::Char('k') => app.palette_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.palette_move_down(),
                        KeyCode::Char(c) => app.palette_push_char(c),
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
                        KeyCode::Char('K') => app.select_move_up(),
                        KeyCode::Char('J') => app.select_move_down(),
                        KeyCode::Left | KeyCode::Char('h') => app.go_parent(),
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                            app.enter_selected()
                        }
                        KeyCode::Char('g') => app.go_top(),
                        KeyCode::Char('G') => app.go_bottom(),
                        KeyCode::Char('~') => app.go_home(),
                        KeyCode::Char('.') => app.toggle_hidden(),
                        KeyCode::Char('i') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.toggle_gitignored()
                        }
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
                        // Quick single-file rename
                        KeyCode::Char('n') | KeyCode::F(2) => app.begin_quick_rename(),
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
                        // Open in terminal editor ($VISUAL → $EDITOR → vi)
                        KeyCode::Char('o') => {
                            if let Some(path) = app.selected_file_path() {
                                let editor = std::env::var("VISUAL")
                                    .or_else(|_| std::env::var("EDITOR"))
                                    .unwrap_or_else(|_| "vi".to_string());

                                // Tear down TUI so the editor owns the terminal.
                                disable_raw_mode()?;
                                execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

                                let status =
                                    std::process::Command::new(&editor).arg(&path).status();

                                // Always restore the TUI, even if the editor failed.
                                enable_raw_mode()?;
                                execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
                                terminal.clear()?;

                                // Refresh listing in case the editor created/deleted files.
                                app.load_dir();

                                match status {
                                    Ok(_) => {
                                        app.status_message =
                                            Some(format!("Returned from {}", editor))
                                    }
                                    Err(e) => {
                                        app.status_message = Some(format!(
                                            "Failed to open editor '{}': {}",
                                            editor, e
                                        ))
                                    }
                                }
                            }
                        }
                        // Path jump bar
                        KeyCode::Char('e') => app.begin_path_jump(),
                        // Open command palette
                        KeyCode::Char(':') => app.open_palette(),
                        // Open with system default (open on macOS, xdg-open on Linux)
                        KeyCode::Char('O') => {
                            if let Some(path) = app.selected_path() {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| path.to_string_lossy().into_owned());

                                #[cfg(target_os = "macos")]
                                let opener = "open";
                                #[cfg(not(target_os = "macos"))]
                                let opener = "xdg-open";

                                match std::process::Command::new(opener).arg(&path).spawn() {
                                    Ok(_) => {
                                        app.status_message = Some(format!(
                                            "Opening {} with system default\u{2026}",
                                            name
                                        ))
                                    }
                                    Err(e) => {
                                        app.status_message =
                                            Some(format!("Failed to open '{}': {}", name, e))
                                    }
                                }
                            }
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

/// Dispatch a palette ActionId to the corresponding App method.
///
/// Actions that require terminal teardown (open-in-editor) are handled here
/// because `events.rs` owns the terminal handle.
fn execute_palette_action(
    app: &mut App,
    action: crate::app::palette::ActionId,
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<()> {
    use crate::app::palette::ActionId;
    match action {
        ActionId::GoHome => app.go_home(),
        ActionId::GoTop => app.go_top(),
        ActionId::GoBottom => app.go_bottom(),
        ActionId::HistoryBack => app.history_back(),
        ActionId::HistoryForward => app.history_forward(),
        ActionId::ToggleHidden => app.toggle_hidden(),
        ActionId::ToggleGitignored => app.toggle_gitignored(),
        ActionId::ToggleDiffPreview => app.toggle_diff_preview(),
        ActionId::ToggleMetaPreview => app.toggle_meta_preview(),
        ActionId::RefreshGitStatus => app.refresh_git_status(),
        ActionId::StartSearch => app.start_search(),
        ActionId::StartFilter => app.start_filter(),
        ActionId::StartContentSearch => app.start_content_search(),
        ActionId::StartFind => app.start_find(),
        ActionId::ClipboardCopyCurrent => app.clipboard_copy_current(),
        ActionId::ClipboardCopySelected => app.clipboard_copy_selected(),
        ActionId::ClipboardCutCurrent => app.clipboard_cut_current(),
        ActionId::PasteClipboard => app.paste_clipboard(),
        ActionId::BeginDeleteCurrent => app.begin_delete_current(),
        ActionId::BeginDeleteSelected => app.begin_delete_selected(),
        ActionId::BeginMkdir => app.begin_mkdir(),
        ActionId::UndoTrash => app.undo_trash(),
        ActionId::BeginChmod => app.begin_chmod(),
        ActionId::SelectMoveDown => app.select_move_down(),
        ActionId::SelectMoveUp => app.select_move_up(),
        ActionId::ToggleSelection => {
            let s = app.selected;
            app.toggle_selection(s);
        }
        ActionId::SelectAll => app.select_all(),
        ActionId::ClearSelections => app.clear_selections(),
        ActionId::QuickRename => app.begin_quick_rename(),
        ActionId::StartRename => app.start_rename(),
        ActionId::AddBookmark => app.add_bookmark(),
        ActionId::OpenBookmarks => app.open_bookmarks(),
        ActionId::CycleSortMode => app.cycle_sort_mode(),
        ActionId::ToggleSortOrder => app.toggle_sort_order(),
        ActionId::YankRelativePath => app.yank_relative_path(),
        ActionId::YankAbsolutePath => app.yank_absolute_path(),
        ActionId::PathJump => app.begin_path_jump(),
        ActionId::ShowHelp => app.show_help = true,
        // Quit appears in the palette for discoverability but cannot break out
        // of the event loop from here — use q directly.
        ActionId::Quit => {}
    }
    // terminal is available here for any future actions needing TUI teardown.
    let _ = terminal;
    Ok(())
}
