use crate::app::App;
use crate::icons::icon_for_entry;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Main draw function. Computes pane layout from app's divider fractions,
/// then renders parent pane, current-dir pane, and preview pane.
pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();
    app.term_width = size.width;
    app.term_height = size.height;

    // Reserve bottom row for search bar or status message.
    let has_bottom_bar = app.search_mode || app.status_message.is_some();
    let (main_area, bottom_area) = if has_bottom_bar && size.height > 3 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(size);
        (chunks[0], Some(chunks[1]))
    } else {
        (size, None)
    };

    // Compute column positions of the two dividers.
    let left_cols = ((app.left_div * main_area.width as f64).round() as u16).max(3);
    let right_cols = ((app.right_div * main_area.width as f64).round() as u16).max(left_cols + 4);
    let mid_cols = right_cols.saturating_sub(left_cols);
    let preview_cols = main_area.width.saturating_sub(right_cols);

    app.left_div_col = left_cols;
    app.right_div_col = right_cols;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_cols),
            Constraint::Length(mid_cols),
            Constraint::Length(preview_cols),
        ])
        .split(main_area);

    // Store preview area for mouse hit-testing.
    app.preview_area = (chunks[2].x, chunks[2].y, chunks[2].width, chunks[2].height);

    draw_parent_pane(f, app, chunks[0]);
    draw_current_pane(f, app, chunks[1]);
    draw_preview_pane(f, app, chunks[2]);

    // Draw divider grab handles.
    draw_divider(f, app.left_div_col.saturating_sub(1), main_area.height, app.drag == Some(crate::app::DragTarget::LeftDivider));
    draw_divider(f, app.right_div_col.saturating_sub(1), main_area.height, app.drag == Some(crate::app::DragTarget::RightDivider));

    // Draw bottom bar.
    if let Some(bar) = bottom_area {
        if app.search_mode {
            draw_search_bar(f, app, bar);
        } else if let Some(ref msg) = app.status_message {
            let para = Paragraph::new(Line::from(Span::styled(
                msg.as_str(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            f.render_widget(para, bar);
        }
    }
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let match_count = app.filtered_indices.len();
    let total = app.entries.len();
    let para = Paragraph::new(Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(
            &app.search_query,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" [{}/{}]", match_count, total),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_divider(f: &mut Frame, col: u16, height: u16, active: bool) {
    let style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    for row in 0..height {
        let area = Rect::new(col, row, 1, 1);
        let span = Span::styled("│", style);
        f.render_widget(Paragraph::new(Line::from(span)), area);
    }
}

fn draw_parent_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .cwd
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string());

    let items: Vec<ListItem> = app
        .parent_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == app.parent_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
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
            .borders(Borders::RIGHT)
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

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_match = !is_searching || app.filtered_indices.contains(&i);
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else if !is_match {
                Style::default().fg(Color::DarkGray)
            } else if entry.is_dir {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = icon_for_entry(&entry.name, entry.is_dir);
            let name = format!("{} {}", icon, entry.name);
            ListItem::new(Span::styled(name, style))
        })
        .collect();

    let info = format!(" {}/{} ", app.selected + 1, app.entries.len());
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::RIGHT)
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

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title)
            .title_bottom(Line::from(scroll_info).right_aligned()),
    );
    f.render_widget(para, area);
}
