use crate::app::{format_size, App};
use crate::icons::icon_for_entry;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Main draw function. Computes pane layout from app's divider fractions,
/// then renders parent pane, current-dir pane, and preview pane.
pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();
    app.term_width = size.width;
    app.term_height = size.height;

    // Reserve 1 row for path bar at top, 1 row for status/search at bottom.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // path bar
            Constraint::Min(3),   // main panes
            Constraint::Length(1), // status / search bar
        ])
        .split(size);

    let path_area = chunks[0];
    let main_area = chunks[1];
    let bottom_area = chunks[2];

    // Draw path bar.
    draw_path_bar(f, app, path_area);

    // Compute column positions of the two dividers.
    let left_cols = ((app.left_div * main_area.width as f64).round() as u16).max(3);
    let right_cols = ((app.right_div * main_area.width as f64).round() as u16).max(left_cols + 4);
    let mid_cols = right_cols.saturating_sub(left_cols);
    let preview_cols = main_area.width.saturating_sub(right_cols);

    app.left_div_col = main_area.x + left_cols;
    app.right_div_col = main_area.x + right_cols;

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_cols),
            Constraint::Length(mid_cols),
            Constraint::Length(preview_cols),
        ])
        .split(main_area);

    // Store pane areas for mouse hit-testing.
    app.parent_area = (pane_chunks[0].x, pane_chunks[0].y, pane_chunks[0].width, pane_chunks[0].height);
    app.current_area = (pane_chunks[1].x, pane_chunks[1].y, pane_chunks[1].width, pane_chunks[1].height);
    app.preview_area = (pane_chunks[2].x, pane_chunks[2].y, pane_chunks[2].width, pane_chunks[2].height);

    // Ensure selection is visible before drawing.
    app.ensure_visible(pane_chunks[1].height);
    app.ensure_parent_visible(pane_chunks[0].height);

    draw_parent_pane(f, app, pane_chunks[0]);
    draw_current_pane(f, app, pane_chunks[1]);
    draw_preview_pane(f, app, pane_chunks[2]);

    // Draw bottom bar.
    if app.search_mode {
        draw_search_bar(f, app, bottom_area);
    } else if let Some(ref msg) = app.status_message {
        let para = Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        f.render_widget(para, bottom_area);
    } else {
        // Show hint.
        let hint = Paragraph::new(Line::from(Span::styled(
            " Press ? for help",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(hint, bottom_area);
    }

    // Help overlay.
    if app.show_help {
        draw_help_overlay(f, size);
    }
}

fn draw_path_bar(f: &mut Frame, app: &App, area: Rect) {
    let path_str = app.cwd.to_string_lossy();
    let hidden_indicator = if app.show_hidden { " [H]" } else { "" };
    let text = format!(" {}{}", path_str, hidden_indicator);
    let para = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    f.render_widget(para, area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let match_count = app.filtered_indices.len();
    let total = app.entries.len();
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "/",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.search_query,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "\u{2588}", // block cursor
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!(" [{}/{}]", match_count, total),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_parent_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .cwd
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string());

    let visible_height = area.height.saturating_sub(1) as usize; // 1 for top title line (no bottom border)
    let items: Vec<ListItem> = app
        .parent_entries
        .iter()
        .enumerate()
        .skip(app.parent_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let style = if i == app.parent_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = icon_for_entry(&entry.name, entry.is_dir);
            let name = format!("{} {}", icon, entry.name);
            ListItem::new(Span::styled(name, style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title),
    );
    f.render_widget(list, area);
}

fn draw_current_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.cwd.to_string_lossy().into_owned());

    let is_searching = app.search_mode && !app.search_query.is_empty();

    let inner_width = area.width.saturating_sub(1) as usize; // 1 col for right border
    let visible_height = area.height.saturating_sub(2) as usize; // top title + bottom info
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .skip(app.current_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let is_match = !is_searching || app.filtered_set.contains(&i);
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else if !is_match {
                Style::default().fg(Color::DarkGray)
            } else if entry.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = icon_for_entry(&entry.name, entry.is_dir);
            let size_str = if entry.is_dir {
                String::new()
            } else {
                format_size(entry.size)
            };
            // Compute padding so size is right-aligned.
            let left_part = format!("{} {}", icon, entry.name);
            let padding = if inner_width > left_part.len() + size_str.len() {
                inner_width - left_part.len() - size_str.len()
            } else {
                1
            };
            let name = format!("{}{:>pad$}{}", left_part, "", size_str, pad = padding);
            ListItem::new(Span::styled(name, style))
        })
        .collect();

    let info = format!(" {}/{} ", app.selected + 1, app.entries.len());
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title)
            .title_bottom(Line::from(info).right_aligned()),
    );
    f.render_widget(list, area);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .entries
        .get(app.selected)
        .map(|e| e.name.clone())
        .unwrap_or_default();

    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.preview_lines.len();
    let scroll_info = if total > 0 {
        let end = (app.preview_scroll + visible_height).min(total);
        format!(" {}-{}/{} ", app.preview_scroll + 1, end, total)
    } else {
        String::new()
    };

    let lines: Vec<Line> = app
        .preview_lines
        .iter()
        .skip(app.preview_scroll)
        .take(visible_height)
        .map(|l| Line::from(l.as_str()))
        .collect();

    // Draw main content (leave 1 col for scrollbar).
    let content_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title)
            .title_bottom(Line::from(scroll_info).right_aligned()),
    );
    f.render_widget(para, content_area);

    // Draw scrollbar in the rightmost column.
    if total > visible_height && visible_height > 0 {
        let scrollbar_col = area.x + area.width - 1;
        let bar_top = area.y + 1; // skip top border
        let bar_height = area.height.saturating_sub(2) as usize; // skip borders

        if bar_height > 0 {
            // Calculate thumb position and size.
            let thumb_size = ((visible_height as f64 / total as f64) * bar_height as f64)
                .ceil()
                .max(1.0) as usize;
            let max_thumb_pos = bar_height.saturating_sub(thumb_size);
            let max_scroll = total.saturating_sub(visible_height);
            let thumb_pos = if max_scroll > 0 {
                ((app.preview_scroll as f64 / max_scroll as f64) * max_thumb_pos as f64).round()
                    as usize
            } else {
                0
            };

            for row_offset in 0..bar_height {
                let ch = if row_offset >= thumb_pos && row_offset < thumb_pos + thumb_size {
                    "\u{2588}" // full block for thumb
                } else {
                    "\u{2591}" // light shade for track
                };
                let r = Rect::new(scrollbar_col, bar_top + row_offset as u16, 1, 1);
                let style = if row_offset >= thumb_pos && row_offset < thumb_pos + thumb_size {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Rgb(40, 40, 40))
                };
                f.render_widget(Paragraph::new(Line::from(Span::styled(ch, style))), r);
            }
        }
    }
}

fn draw_help_overlay(f: &mut Frame, size: Rect) {
    let width = 50u16.min(size.width.saturating_sub(4));
    let height = 22u16.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let help_lines = vec![
        Line::from(Span::styled(
            " Keybindings",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/Down    ", Style::default().fg(Color::Cyan)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("  k/Up      ", Style::default().fg(Color::Cyan)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("  l/Right   ", Style::default().fg(Color::Cyan)),
            Span::raw("Enter dir / yank file path"),
        ]),
        Line::from(vec![
            Span::styled("  h/Left    ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to parent"),
        ]),
        Line::from(vec![
            Span::styled("  Enter     ", Style::default().fg(Color::Cyan)),
            Span::raw("Enter dir / yank file path"),
        ]),
        Line::from(vec![
            Span::styled("  g         ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to top"),
        ]),
        Line::from(vec![
            Span::styled("  G         ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to bottom"),
        ]),
        Line::from(vec![
            Span::styled("  ~         ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to home directory"),
        ]),
        Line::from(vec![
            Span::styled("  .         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle hidden files"),
        ]),
        Line::from(vec![
            Span::styled("  /         ", Style::default().fg(Color::Cyan)),
            Span::raw("Fuzzy search"),
        ]),
        Line::from(vec![
            Span::styled("  y         ", Style::default().fg(Color::Cyan)),
            Span::raw("Yank relative path"),
        ]),
        Line::from(vec![
            Span::styled("  Y         ", Style::default().fg(Color::Cyan)),
            Span::raw("Yank absolute path"),
        ]),
        Line::from(vec![
            Span::styled("  Q         ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Drag dividers with the mouse to resize panes",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Scroll wheel works on all panes",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Help ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let para = Paragraph::new(help_lines).block(block);
    f.render_widget(para, area);
}
