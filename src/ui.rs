use crate::app::{format_size, App, SortMode, SortOrder};
use crate::git::FileStatus;
use crate::icons::icon_for_entry;
use crate::ops::ClipboardOp;
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
    // mkdir mode also needs 1 row (reuses the 1-row bottom area).
    // delete confirmation and mkdir both fit in 1 row.
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
    } else if app.find_mode {
        draw_find_pane(f, app, pane_chunks[1]);
    } else {
        draw_current_pane(f, app, pane_chunks[1]);
    }
    draw_preview_pane(f, app, pane_chunks[2]);

    // Draw bottom bar.
    if app.rename_mode {
        draw_rename_bar(f, app, bottom_area);
    } else if !app.pending_delete.is_empty() {
        draw_delete_confirm_bar(f, app, bottom_area);
    } else if app.mkdir_mode {
        draw_mkdir_bar(f, app, bottom_area);
    } else if app.chmod_mode {
        draw_chmod_bar(f, app, bottom_area);
    } else if app.content_search_mode {
        draw_content_search_bar(f, app, bottom_area);
    } else if app.find_mode {
        draw_find_bar(f, app, bottom_area);
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
    } else if let Some(ref clip) = app.clipboard {
        // Show clipboard indicator.
        let (label, color) = match clip.op {
            ClipboardOp::Copy => ("[copy]", Color::Green),
            ClipboardOp::Cut => ("[cut]", Color::Yellow),
        };
        let count = clip.paths.len();
        let para = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", label),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} file{}  — p: paste",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
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

    // Bookmark picker overlay (rendered on top of everything else).
    if app.bookmark_mode {
        draw_bookmark_overlay(f, app, size);
    }
}

fn draw_path_bar(f: &mut Frame, app: &App, area: Rect) {
    // Smart path truncation: keep last 3 components when path is wide.
    let available = area.width.saturating_sub(4) as usize; // rough margin
    let path_str = app.cwd.to_string_lossy();
    let display_path = if path_str.chars().count() > available && available > 4 {
        // Keep last 3 path components with …/ prefix.
        let components: Vec<&str> = path_str.split('/').filter(|c| !c.is_empty()).collect();
        let keep = components.len().min(3);
        let tail: Vec<&str> = components[components.len() - keep..].to_vec();
        format!("…/{}", tail.join("/"))
    } else {
        path_str.into_owned()
    };

    let mut spans = vec![Span::styled(
        format!(" {}", display_path),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];

    // Hidden files indicator as a separate, dimmed span.
    if app.show_hidden {
        spans.push(Span::styled("  [H]", Style::default().fg(Color::DarkGray)));
    }

    if let Some(branch) = app.git_status.as_ref().and_then(|g| g.branch.as_ref()) {
        spans.push(Span::styled(
            format!("  ({})", branch),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Show sort indicator when not using the default (Name ascending).
    if app.sort_mode != SortMode::Name || app.sort_order != SortOrder::Ascending {
        let arrow = if app.sort_order == SortOrder::Descending {
            "↓"
        } else {
            "↑"
        };
        spans.push(Span::styled(
            format!("  {} {}", arrow, app.sort_mode.label()),
            Style::default().fg(Color::DarkGray),
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

fn draw_delete_confirm_bar(f: &mut Frame, app: &App, area: Rect) {
    let count = app.pending_delete.len();
    let subject = if count == 1 {
        app.pending_delete
            .first()
            .and_then(|p| p.file_name())
            .map(|n| format!("\"{}\"", n.to_string_lossy()))
            .unwrap_or_else(|| "1 item".to_string())
    } else {
        format!("{} items", count)
    };
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" Trash {}? ", subject),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[t/y]", Style::default().fg(Color::Green)),
        Span::styled("trash  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[D]", Style::default().fg(Color::Red)),
        Span::styled("delete permanently  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::styled("cancel", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(para, area);
}

fn draw_chmod_bar(f: &mut Frame, app: &App, area: Rect) {
    // Show the current octal mode as context.
    let current = app
        .entries
        .get(app.selected)
        .and_then(|e| std::fs::metadata(&e.path).ok())
        .map(|m| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                format!("{:04o}", m.permissions().mode() & 0o7777)
            }
            #[cfg(not(unix))]
            {
                "????".to_string()
            }
        })
        .unwrap_or_default();

    let name = app
        .entries
        .get(app.selected)
        .map(|e| e.name.as_str())
        .unwrap_or("");

    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" chmod {} [current: {}]: ", name, current),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(app.chmod_input.as_str(), Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::Yellow)),
        Span::styled(
            "  Enter=apply  Esc=cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_mkdir_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "New directory: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.mkdir_input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Enter: create   Esc: cancel",
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

    let inner_width = area.width.saturating_sub(2) as usize; // account for right border
    let visible_height = area.height.saturating_sub(1) as usize;
    let items: Vec<ListItem> = app
        .parent_entries
        .iter()
        .enumerate()
        .skip(app.parent_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let style = if i == app.parent_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = icon_for_entry(&entry.name, entry.is_dir);
            let raw_name = format!("{} {}", icon, entry.name);
            let name = truncate_with_ellipsis(&raw_name, inner_width.saturating_sub(1));
            ListItem::new(Span::styled(name, style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(title, Style::default().fg(Color::DarkGray))),
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
    // 2-char prefix always reserved so layout doesn't shift when selection changes.
    let sel_prefix_width: usize = 2;
    let has_selection = !app.rename_selected.is_empty();

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
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if is_marked {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else if !is_match {
                Style::default().fg(Color::DarkGray)
            } else if entry.is_dir {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Determine git status indicator (char, color) for this entry.
            let git_indicator: Option<(char, Color)> = app.git_status.as_ref().and_then(|git| {
                if entry.is_dir {
                    if git.subtree_dirty(&entry.path) {
                        Some(('\u{25cf}', Color::Yellow)) // ● dimmed for dirty dir
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
            let indicator_width: usize = if git_indicator.is_some() { 2 } else { 0 };
            let size_width = size_str.len();
            // Available space for icon+name after fixed columns.
            let max_name_width =
                inner_width.saturating_sub(sel_prefix_width + size_width + indicator_width + 1);
            let left_part_raw = format!("{} {}", icon, entry.name);
            let left_part = truncate_with_ellipsis(&left_part_raw, max_name_width);
            let total_fixed =
                sel_prefix_width + left_part.chars().count() + size_width + indicator_width;
            let padding = if inner_width > total_fixed {
                inner_width - total_fixed
            } else {
                1
            };

            let mut spans: Vec<Span> = Vec::new();

            // Selection prefix always rendered (2 chars) to prevent layout shift.
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
            let _ = has_selection; // prefix always shown; variable retained for clarity
            spans.push(Span::styled(mark, mark_style));

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
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(ch.to_string(), ind_style));
                spans.push(Span::styled(" ", style));
            }

            // Size rendered in dimmer style to visually separate it from the name.
            if !size_str.is_empty() {
                let size_style = if is_cursor {
                    Style::default().fg(Color::Gray).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(size_str, size_style));
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
            .borders(Borders::TOP | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_bottom(
                Line::from(Span::styled(info, Style::default().fg(Color::DarkGray)))
                    .right_aligned(),
            ),
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
            } else if app.meta_preview_mode {
                format!("{} [meta]", e.name)
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

    // Try syntax highlighting for source files (non-diff mode only).
    let highlighted: Option<Vec<Line<'static>>> = if !app.preview_is_diff {
        app.entries.get(app.selected).and_then(|e| {
            let ext = std::path::Path::new(&e.name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext.is_empty() {
                None
            } else {
                let max_process =
                    (app.preview_scroll + visible_height).min(app.preview_lines.len());
                app.highlighter
                    .highlight(&app.preview_lines[..max_process], ext, max_process)
            }
        })
    } else {
        None
    };

    let lines: Vec<Line> = if let Some(hl) = highlighted {
        hl.into_iter()
            .skip(app.preview_scroll)
            .take(visible_height)
            .collect()
    } else {
        app.preview_lines
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
            .collect()
    };

    // Draw main content (leave 1 col for scrollbar).
    let content_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(title, Style::default().fg(Color::DarkGray)))
            .title_bottom(
                Line::from(Span::styled(
                    scroll_info,
                    Style::default().fg(Color::DarkGray),
                ))
                .right_aligned(),
            ),
    );
    f.render_widget(para, content_area);

    // Draw scrollbar in the rightmost column (always show track when content overflows).
    if total > visible_height && visible_height > 0 {
        let scrollbar_col = area.x + area.width - 1;
        let bar_top = area.y + 1; // skip top border
        let bar_height = area.height.saturating_sub(1) as usize; // only skip top border

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
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::Rgb(60, 60, 60))
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
                    .fg(Color::Cyan)
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
                        .fg(Color::White)
                        .bg(Color::Blue)
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
        FileStatus::Conflict => ('\u{2716}', Color::Red), // ✖
        FileStatus::Deleted => ('\u{2716}', Color::Red),  // ✖
        FileStatus::Staged | FileStatus::StagedModified => ('\u{271a}', Color::Green), // ✚
        FileStatus::Modified => ('\u{25cf}', Color::Yellow), // ●
        FileStatus::Untracked => ('+', Color::Cyan),
    }
}

/// Truncate `s` to `max_chars` characters, appending `…` when truncated.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
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

fn section_header(label: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", label),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_line(key: &'static str, desc: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<10}", key), Style::default().fg(Color::Cyan)),
        Span::raw(desc),
    ])
}

/// Render recursive find results in the center pane during find mode.
fn draw_find_pane(f: &mut Frame, app: &App, area: Rect) {
    let count = app.find_results.len();
    let title = if app.find_query.is_empty() {
        " Find files ".to_string()
    } else if count == 0 {
        " No matches ".to_string()
    } else {
        let trunc = if app.find_truncated {
            " [truncated]"
        } else {
            ""
        };
        format!(
            " {} file{}{} ",
            count,
            if count == 1 { "" } else { "s" },
            trunc
        )
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = if app.find_selected >= visible_height {
        app.find_selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .find_results
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, r)| {
            let is_selected = i == app.find_selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {}", r.relative.display()),
                style,
            )))
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

/// Render the bookmark picker as a centered overlay.
fn draw_bookmark_overlay(f: &mut Frame, app: &App, size: Rect) {
    // Compute overlay dimensions.  Minimum usable height is 6 rows.
    let width = 62u16.min(size.width.saturating_sub(4));
    let max_rows = app.bookmark_filtered.len().max(1) as u16;
    let height = (max_rows + 4).min(size.height.saturating_sub(4)).max(6);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    // Title: show filter query when active.
    let title = if app.bookmark_query.is_empty() {
        " Bookmarks ".to_string()
    } else {
        format!(" Bookmarks  {} ", app.bookmark_query)
    };

    // Hint in title right section — put it in the title for simplicity.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let name_col = 16usize; // chars reserved for short name

    if app.bookmark_filtered.is_empty() {
        let msg = if app.bookmarks.is_empty() {
            "  No bookmarks — press b to add one"
        } else {
            "  No matches"
        };
        let para = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(para, inner);
        return;
    }

    let scroll = if app.bookmark_selected >= visible_height {
        app.bookmark_selected - visible_height + 1
    } else {
        0
    };

    let path_width = (inner.width as usize).saturating_sub(name_col + 2);

    let items: Vec<ListItem> = app
        .bookmark_filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(display_idx, &real_idx)| {
            let path = &app.bookmarks[real_idx];
            let exists = path.is_dir();
            let is_selected = display_idx == app.bookmark_selected;

            // Short name = last path component.
            let short = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let short_col = truncate_with_ellipsis(&short, name_col);

            let full = path.to_string_lossy().into_owned();
            // Replace $HOME with ~
            let home = std::env::var("HOME").unwrap_or_default();
            let display_path = if !home.is_empty() && full.starts_with(&home) {
                format!("~{}", &full[home.len()..])
            } else {
                full
            };
            let path_col = truncate_with_ellipsis(&display_path, path_width);

            let gone_suffix = if exists { "" } else { "  [gone]" };

            if is_selected {
                let style = Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(format!("{}{}", path_col, gone_suffix), style),
                ]))
            } else if !exists {
                let style = Style::default().fg(Color::DarkGray);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(format!("{}{}", path_col, gone_suffix), style),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<width$}", short_col, width = name_col),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(path_col),
                ]))
            }
        })
        .collect();

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            "  Enter",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": jump  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "d",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": remove  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": close", Style::default().fg(Color::DarkGray)),
    ]));

    // Split inner area: list rows above, hint row at bottom.
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let list = List::new(items);
    f.render_widget(list, inner_chunks[0]);
    f.render_widget(hint, inner_chunks[1]);
}

/// Render the find prompt in the status bar.
fn draw_find_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref err) = app.find_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!(" \u{26a0} {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(para, area);
        return;
    }
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Find: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(app.find_query.as_str(), Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
    ]));
    f.render_widget(para, area);
}

fn draw_help_overlay(f: &mut Frame, size: Rect) {
    let width = 60u16.min(size.width.saturating_sub(4));
    let height = 46u16.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let help_lines = vec![
        // ── Navigation ──────────────────────────────────────────────────────
        section_header("Navigation"),
        key_line("j/Down", "Move down"),
        key_line("k/Up", "Move up"),
        key_line("l/Right", "Enter dir / yank file path"),
        key_line("h/Left", "Go to parent"),
        key_line("Enter", "Enter dir / yank file path"),
        key_line("g / G", "Go to top / bottom"),
        key_line("~", "Go to home directory"),
        key_line(".", "Toggle hidden files"),
        key_line("Ctrl+O", "Go back in directory history"),
        key_line("Ctrl+I", "Go forward in directory history"),
        Line::from(""),
        // ── Search ──────────────────────────────────────────────────────────
        section_header("Search"),
        key_line("/", "Fuzzy search"),
        key_line("Ctrl+F", "Content search (ripgrep)"),
        key_line("Ctrl+P", "Recursive filename find"),
        key_line("b", "Bookmark current directory"),
        key_line("B", "Open bookmark picker"),
        Line::from(""),
        // ── View ────────────────────────────────────────────────────────────
        section_header("View"),
        key_line("d", "Toggle git diff preview"),
        key_line("m", "Toggle file metadata view"),
        key_line("P", "Edit file permissions (chmod)"),
        key_line("R", "Refresh git status"),
        key_line("S", "Cycle sort: Name/Size/Modified/Ext"),
        key_line("s", "Toggle sort order ↑↓"),
        Line::from(""),
        // ── Selection & Rename ──────────────────────────────────────────────
        section_header("Selection & Rename"),
        key_line("Space", "Toggle file selection"),
        key_line("v", "Select all files"),
        key_line("r", "Rename selected files"),
        key_line("Esc", "Clear selections / cancel mode"),
        Line::from(""),
        // ── File Operations ─────────────────────────────────────────────────
        section_header("File Operations"),
        key_line("c / C", "Copy current / selected"),
        key_line("x", "Cut current to clipboard"),
        key_line("p", "Paste clipboard into current dir"),
        key_line("Delete / X", "Trash current / selected (recoverable)"),
        key_line("u", "Undo last trash operation"),
        key_line("M", "Make new directory"),
        Line::from(""),
        // ── Yank & Misc ─────────────────────────────────────────────────────
        section_header("Yank & Misc"),
        key_line("y / Y", "Yank relative / absolute path"),
        key_line("Q", "Quit"),
        key_line("?", "Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "  Drag dividers to resize · scroll wheel on all panes",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
    let para = Paragraph::new(help_lines).block(block);
    f.render_widget(para, area);
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Given: a string shorter than max_chars
    /// When: truncate_with_ellipsis is called
    /// Then: the original string is returned unchanged
    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    /// Given: a string exactly equal to max_chars
    /// When: truncate_with_ellipsis is called
    /// Then: the original string is returned (no ellipsis)
    #[test]
    fn truncate_at_limit_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    /// Given: a string longer than max_chars
    /// When: truncate_with_ellipsis is called
    /// Then: the result ends with '…' and is at most max_chars chars long
    #[test]
    fn truncate_long_string_appends_ellipsis() {
        let result = truncate_with_ellipsis("hello world", 8);
        assert!(result.ends_with('…'), "expected ellipsis: {result}");
        assert!(result.chars().count() <= 8, "expected <= 8 chars: {result}");
    }

    /// Given: max_chars = 1
    /// When: truncate_with_ellipsis is called on a longer string
    /// Then: result is just '…'
    #[test]
    fn truncate_min_width_returns_ellipsis_only() {
        let result = truncate_with_ellipsis("hello", 1);
        assert_eq!(result, "…");
    }

    /// Given: a string with multi-byte Unicode characters
    /// When: truncate_with_ellipsis is called
    /// Then: truncation is based on char count, not byte count
    #[test]
    fn truncate_respects_unicode_chars() {
        // "café" = 4 chars but 5 bytes (UTF-8)
        let result = truncate_with_ellipsis("café world", 5);
        assert!(result.chars().count() <= 5);
        assert!(result.ends_with('…'));
    }
}
