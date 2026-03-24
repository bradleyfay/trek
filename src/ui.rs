use crate::app::{format_size, App};
use crate::git::FileStatus;
use crate::icons::icon_for_entry;
use crate::rename::{RenameField, RenameResult};
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
    // Rename mode needs 3 rows at the bottom (pattern, replacement, error/hint).
    let bottom_height: u16 = if app.rename_mode { 3 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),             // path bar
            Constraint::Min(3),                // main panes
            Constraint::Length(bottom_height), // status / search / rename bar
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

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_cols),
            Constraint::Length(mid_cols),
            Constraint::Length(preview_cols),
        ])
        .split(main_area);

    // Store computed geometry so mouse handlers can do hit-testing.
    app.apply_layout(
        size.width,
        size.height,
        main_area.x + left_cols,
        main_area.x + right_cols,
        (
            pane_chunks[0].x,
            pane_chunks[0].y,
            pane_chunks[0].width,
            pane_chunks[0].height,
        ),
        (
            pane_chunks[1].x,
            pane_chunks[1].y,
            pane_chunks[1].width,
            pane_chunks[1].height,
        ),
        (
            pane_chunks[2].x,
            pane_chunks[2].y,
            pane_chunks[2].width,
            pane_chunks[2].height,
        ),
    );

    // Ensure selection is visible before drawing.
    app.ensure_visible(pane_chunks[1].height);
    app.ensure_parent_visible(pane_chunks[0].height);

    draw_parent_pane(f, app, pane_chunks[0]);
    if app.rename_mode {
        draw_rename_preview_pane(f, app, pane_chunks[1]);
    } else if app.content_search_mode {
        draw_content_search_pane(f, app, pane_chunks[1]);
    } else {
        draw_current_pane(f, app, pane_chunks[1]);
    }
    draw_preview_pane(f, app, pane_chunks[2]);

    // Draw bottom bar.
    if app.rename_mode {
        draw_rename_bar(f, app, bottom_area);
    } else if app.content_search_mode {
        draw_content_search_bar(f, app, bottom_area);
    } else if app.search_mode {
        draw_search_bar(f, app, bottom_area);
    } else if let Some(ref msg) = app.status_message {
        let para = Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        f.render_widget(para, bottom_area);
    } else if !app.rename_selected.is_empty() {
        let count = app.rename_selected.len();
        let para = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} selected", count),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  — r: rename   v: all   Esc: clear",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
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
    let mut spans = vec![Span::styled(
        format!(" {}{}", path_str, hidden_indicator),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(branch) = app.git_status.as_ref().and_then(|g| g.branch.as_ref()) {
        spans.push(Span::styled(
            format!("  ({})", branch),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
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
    let has_selection = !app.rename_selected.is_empty();
    // 2 chars for "✓ " / "  " prefix when any file is selected.
    let sel_prefix_width: usize = if has_selection { 2 } else { 0 };

    let inner_width = area.width.saturating_sub(1) as usize; // 1 col for right border
    let visible_height = area.height.saturating_sub(2) as usize; // top title + bottom info
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .skip(app.current_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let is_cursor = i == app.selected;
            let is_marked = app.rename_selected.contains(&i);
            let is_match = !is_searching || app.filtered_set.contains(&i);
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_marked {
                Style::default()
                    .fg(Color::Magenta)
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

            // Determine git status indicator (char, color) for this entry.
            let git_indicator: Option<(char, Color)> = app.git_status.as_ref().and_then(|git| {
                if entry.is_dir {
                    if git.subtree_dirty(&entry.path) {
                        Some(('~', Color::Yellow))
                    } else {
                        None
                    }
                } else {
                    git.for_path(&entry.path).map(file_status_indicator)
                }
            });

            let icon = icon_for_entry(&entry.name, entry.is_dir);
            let size_str = if entry.is_dir {
                String::new()
            } else {
                format_size(entry.size)
            };

            // Layout: "[✓ ]{icon} {name}{padding}[indicator ]{size_str}"
            let left_part = format!("{} {}", icon, entry.name);
            let indicator_width: usize = if git_indicator.is_some() { 2 } else { 0 };
            let total_fixed = sel_prefix_width + left_part.len() + size_str.len() + indicator_width;
            let padding = if inner_width > total_fixed {
                inner_width - total_fixed
            } else {
                1
            };

            let mut spans: Vec<Span> = Vec::new();

            // Selection checkmark prefix (shown when any file is selected).
            if has_selection {
                let (mark, mark_style) = if is_marked {
                    (
                        "✓ ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("  ", Style::default())
                };
                spans.push(Span::styled(mark, mark_style));
            }

            // Icon + name + padding.
            spans.push(Span::styled(
                format!("{}{:>pad$}", left_part, "", pad = padding),
                style,
            ));

            // Git status indicator.
            if let Some((ch, color)) = git_indicator {
                let ind_style = if is_cursor {
                    Style::default()
                        .fg(color)
                        .bg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(ch.to_string(), ind_style));
                spans.push(Span::styled(" ", style));
            }

            if !size_str.is_empty() {
                spans.push(Span::styled(size_str, style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let info = if app.entries_truncated {
        format!(" {}/{} [limit] ", app.selected + 1, app.entries.len())
    } else {
        format!(" {}/{} ", app.selected + 1, app.entries.len())
    };
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
        .map(|e| {
            if app.preview_is_diff {
                format!("{} [diff]", e.name)
            } else {
                e.name.clone()
            }
        })
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
        .map(|l| {
            if app.preview_is_diff {
                colorize_diff_line(l)
            } else {
                Line::from(l.as_str())
            }
        })
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

/// Render the rename live-preview table in the center pane (shown while rename_mode is active).
fn draw_rename_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let match_count = app
        .rename_preview
        .iter()
        .filter(|r| matches!(r.result, RenameResult::Renamed(_)))
        .count();
    let total = app.rename_preview.len();
    let title = format!(" {}/{} matched ", match_count, total);

    let visible_height = area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = app
        .rename_preview
        .iter()
        .take(visible_height)
        .map(|row| match &row.result {
            RenameResult::Renamed(new_name) => ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(row.original.clone(), Style::default()),
                Span::styled("  →  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    new_name.clone(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
            RenameResult::NoMatch => ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(row.original.clone(), Style::default().fg(Color::DarkGray)),
                Span::styled("  [no match]", Style::default().fg(Color::DarkGray)),
            ])),
            RenameResult::Conflict(new_name) => ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(row.original.clone(), Style::default()),
                Span::styled("  →  ", Style::default().fg(Color::DarkGray)),
                Span::styled(new_name.clone(), Style::default().fg(Color::Red)),
                Span::styled(" [conflict]", Style::default().fg(Color::Red)),
            ])),
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title),
    );
    f.render_widget(list, area);
}

/// Render the two-field Pattern/Replacement input bar at the bottom of the screen.
fn draw_rename_bar(f: &mut Frame, app: &App, area: Rect) {
    if area.height < 2 {
        return;
    }
    let pat_area = Rect::new(area.x, area.y, area.width, 1);
    let rep_area = Rect::new(area.x, area.y + 1, area.width, 1);

    let pat_cursor = if app.rename_focus == RenameField::Pattern {
        "\u{2588}"
    } else {
        ""
    };
    let rep_cursor = if app.rename_focus == RenameField::Replacement {
        "\u{2588}"
    } else {
        ""
    };

    let label_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Pattern  : ", label_style),
            Span::styled(
                app.rename_pattern.clone(),
                Style::default().fg(Color::White),
            ),
            Span::styled(pat_cursor, Style::default().fg(Color::White)),
        ])),
        pat_area,
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Replace  : ", label_style),
            Span::styled(
                app.rename_replacement.clone(),
                Style::default().fg(Color::White),
            ),
            Span::styled(rep_cursor, Style::default().fg(Color::White)),
        ])),
        rep_area,
    );

    // Third row: error message or hint (only if space allows).
    if area.height >= 3 {
        let hint_area = Rect::new(area.x, area.y + 2, area.width, 1);
        if let Some(ref err) = app.rename_error {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("  \u{26a0} {}", err),
                    Style::default().fg(Color::Red),
                ))),
                hint_area,
            );
        } else {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  Tab: switch field   Enter: apply   Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                ))),
                hint_area,
            );
        }
    }
}

/// Render grouped rg results in the center pane during content search mode.
fn draw_content_search_pane(f: &mut Frame, app: &App, area: Rect) {
    let total_matches: usize = app
        .content_search_results
        .iter()
        .map(|g| g.matches.len())
        .sum();
    let file_count = app.content_search_results.len();

    let title = if total_matches > 0 {
        let trunc = if app.content_search_truncated {
            " [truncated]"
        } else {
            ""
        };
        format!(
            " {} match{} in {} file{}{} ",
            total_matches,
            if total_matches == 1 { "" } else { "es" },
            file_count,
            if file_count == 1 { "" } else { "s" },
            trunc
        )
    } else if app.content_search_query.is_empty() {
        " Content search ".to_string()
    } else {
        " No matches ".to_string()
    };

    let visible_height = area.height.saturating_sub(2) as usize;

    // Build a flat render list so we can compute a scroll offset that keeps
    // the selected match visible without mutable-variable gymnastics.
    enum RowKind {
        Header(String),
        Match {
            flat_idx: usize,
            line_number: u64,
            content: String,
        },
    }
    let mut flat_rows: Vec<RowKind> = Vec::new();
    let mut flat_idx = 0usize;
    for group in &app.content_search_results {
        flat_rows.push(RowKind::Header(format!(" {}", group.file.display())));
        for m in &group.matches {
            flat_rows.push(RowKind::Match {
                flat_idx,
                line_number: m.line_number,
                content: m.line_content.clone(),
            });
            flat_idx += 1;
        }
    }

    // Find row position of the selected match to compute scroll.
    let selected_row = flat_rows
        .iter()
        .position(|r| matches!(r, RowKind::Match { flat_idx: fi, .. } if *fi == app.content_search_selected))
        .unwrap_or(0);
    let scroll = if selected_row >= visible_height {
        selected_row - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem> = flat_rows
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|row| match row {
            RowKind::Header(text) => ListItem::new(Line::from(Span::styled(
                text.clone(),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ))),
            RowKind::Match {
                flat_idx: fi,
                line_number,
                content,
            } => {
                let is_selected = *fi == app.content_search_selected;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let num_style = if is_selected {
                    style
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("   {:>4}  ", line_number), num_style),
                    Span::styled(content.clone(), style),
                ]))
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title),
    );
    f.render_widget(list, area);
}

/// Render the content search prompt in the status bar.
fn draw_content_search_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref err) = app.content_search_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!(" \u{26a0} {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(para, area);
        return;
    }
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Search contents: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.content_search_query,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Enter: run   Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

/// Map a `FileStatus` to a display character and colour.
fn file_status_indicator(status: FileStatus) -> (char, Color) {
    match status {
        FileStatus::Conflict => ('!', Color::Red),
        FileStatus::Deleted => ('D', Color::Red),
        FileStatus::Staged | FileStatus::StagedModified => ('S', Color::Green),
        FileStatus::Modified => ('M', Color::Yellow),
        FileStatus::Untracked => ('?', Color::Cyan),
    }
}

/// Apply diff-aware colouring to a single diff output line.
fn colorize_diff_line(line: &str) -> Line<'_> {
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(Color::Green)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(Color::Red)
    } else if line.starts_with("@@") {
        Style::default().fg(Color::Cyan)
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    Line::from(Span::styled(line, style))
}

fn draw_help_overlay(f: &mut Frame, size: Rect) {
    let width = 50u16.min(size.width.saturating_sub(4));
    let height = 32u16.min(size.height.saturating_sub(4));
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
            Span::styled("  Ctrl+F    ", Style::default().fg(Color::Cyan)),
            Span::raw("Content search (ripgrep)"),
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
            Span::styled("  d         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle git diff preview"),
        ]),
        Line::from(vec![
            Span::styled("  R         ", Style::default().fg(Color::Cyan)),
            Span::raw("Refresh git status"),
        ]),
        Line::from(vec![
            Span::styled("  Space     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle file selection"),
        ]),
        Line::from(vec![
            Span::styled("  v         ", Style::default().fg(Color::Cyan)),
            Span::raw("Select all files"),
        ]),
        Line::from(vec![
            Span::styled("  r         ", Style::default().fg(Color::Cyan)),
            Span::raw("Rename selected files"),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(Color::Cyan)),
            Span::raw("Clear selections"),
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
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    let para = Paragraph::new(help_lines).block(block);
    f.render_widget(para, area);
}
