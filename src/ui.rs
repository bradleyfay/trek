use crate::app::{format_dir_count, format_listing_date, format_size, App, SortMode, SortOrder};
use crate::git::FileStatus;
use crate::icons::icon_for_entry;
use crate::ops::ClipboardOp;
use crate::rename::{RenameField, RenameResult};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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
    if !app.preview_collapsed {
        draw_preview_pane(f, app, pane_chunks[2]);
    }

    // Draw bottom bar.
    if app.rename_mode {
        draw_rename_bar(f, app, bottom_area);
    } else if !app.pending_delete.is_empty() {
        draw_delete_confirm_bar(f, app, bottom_area);
    } else if let Some(ref path) = app.pending_extract {
        draw_extract_bar(f, bottom_area, path);
    } else if app.archive_create_mode {
        draw_archive_create_bar(f, app, bottom_area);
    } else if app.quick_rename_mode {
        draw_quick_rename_bar(f, app, bottom_area);
    } else if app.path_mode {
        draw_path_jump_bar(f, app, bottom_area);
    } else if app.glob_mode {
        draw_glob_select_bar(f, app, bottom_area);
    } else if app.dup_mode {
        draw_dup_bar(f, app, bottom_area);
    } else if app.symlink_mode {
        draw_symlink_bar(f, app, bottom_area);
    } else if app.mkdir_mode {
        draw_mkdir_bar(f, app, bottom_area);
    } else if app.touch_mode {
        draw_touch_bar(f, app, bottom_area);
    } else if app.chmod_mode {
        draw_chmod_bar(f, app, bottom_area);
    } else if app.content_search_mode {
        draw_content_search_bar(f, app, bottom_area);
    } else if app.find_mode {
        draw_find_bar(f, app, bottom_area);
    } else if app.filter_mode {
        draw_filter_bar(f, app, bottom_area);
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
        let total_bytes: u64 = app
            .rename_selected
            .iter()
            .filter_map(|&i| app.entries.get(i))
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        let size_label = if total_bytes > 0 {
            format!("  ({})", format_size(total_bytes))
        } else {
            String::new()
        };
        let para = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} selected{}", count, size_label),
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

    // Bookmark picker overlay.
    if app.bookmark_mode {
        draw_bookmark_overlay(f, app, size);
    }

    // Frecency jump overlay.
    if app.frecency_mode {
        draw_frecency_overlay(f, app, size);
    }

    // Yank picker overlay.
    if app.yank_picker_mode {
        draw_yank_picker(f, app, size);
    }

    // Clipboard inspector overlay.
    if app.clipboard_inspect_mode {
        draw_clipboard_inspect_overlay(f, app, size);
    }

    // Command palette overlay (rendered on top of everything else).
    if app.palette_mode {
        draw_palette_overlay(f, app, size);
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

    // Gitignore filter badge — shown next to the git branch indicator.
    if app.hide_gitignored {
        spans.push(Span::styled(
            "  [ignore]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
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

    // Watcher indicator — shown when the filesystem watcher is active.
    if app.watcher.is_some() {
        spans.push(Span::styled(
            "  [watch]",
            Style::default()
                .fg(Color::Cyan)
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

fn draw_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Filter: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}_", app.filter_input),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "  Esc=clear  Enter=freeze",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(para, area);
}

fn draw_extract_bar(f: &mut Frame, area: Rect, path: &std::path::Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Extract ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  \"{}\" → ./   ", name)),
        Span::styled("[y/Enter]", Style::default().fg(Color::Green)),
        Span::raw("confirm  "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::styled("cancel", Style::default().fg(Color::DarkGray)),
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

fn draw_quick_rename_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Rename: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.quick_rename_input.as_str(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Enter=confirm  Esc=cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_path_jump_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Jump to: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.path_input.as_str(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Tab=complete  Enter=go  Esc=cancel",
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

fn draw_touch_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "New file: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.touch_input,
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

fn draw_archive_create_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Archive:  ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.archive_create_input,
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

fn draw_dup_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Duplicate: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.dup_input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Enter: copy   Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_symlink_bar(f: &mut Frame, app: &App, area: Rect) {
    let target_name = app
        .symlink_target
        .as_deref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "\u{2026}".to_string());
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("Symlink \u{2192} {} : ", target_name),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.symlink_input,
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

fn draw_glob_select_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Glob select: ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.glob_input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
        Span::styled(
            "  Enter: select   Esc: cancel  (e.g. *.rs  *.log  test_?)",
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
    let base_title = app
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.cwd.to_string_lossy().into_owned());

    // Show [~pattern] when filter is frozen (active but bar is closed).
    let title = if !app.filter_input.is_empty() && !app.filter_mode {
        format!("{} [~{}]", base_title, app.filter_input)
    } else {
        base_title
    };

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
            // Right column priority: timestamps > dir counts > file size.
            let right_col_str: String = if app.show_timestamps {
                if entry.is_dir {
                    String::new()
                } else {
                    format_listing_date(entry.modified)
                }
            } else if entry.is_dir && app.show_dir_counts {
                format_dir_count(entry.child_count)
            } else if entry.is_dir {
                String::new()
            } else {
                format_size(entry.size)
            };

            // Layout: "[✓ ]{icon} {name}{padding}[indicator ]{right_col_str}"
            let indicator_width: usize = if git_indicator.is_some() { 2 } else { 0 };
            let right_col_width = right_col_str.len();
            // Available space for icon+name after fixed columns.
            let max_name_width = inner_width
                .saturating_sub(sel_prefix_width + right_col_width + indicator_width + 1);
            let left_part_raw = format!("{} {}", icon, entry.name);
            let left_part = truncate_with_ellipsis(&left_part_raw, max_name_width);
            let total_fixed =
                sel_prefix_width + left_part.chars().count() + right_col_width + indicator_width;
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

            // Right column rendered in dimmer style to visually separate it from the name.
            if !right_col_str.is_empty() {
                let col_style = if is_cursor {
                    Style::default().fg(Color::Gray).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(right_col_str, col_style));
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
            let mut t = if app.du_preview_mode && e.is_dir {
                format!("{} [du]", e.name)
            } else if app.hex_view_mode {
                format!("{} [hex]", e.name)
            } else if app.preview_is_diff {
                format!("{} [diff]", e.name)
            } else if app.hash_preview_mode {
                format!("{} [hash]", e.name)
            } else if app.meta_preview_mode {
                format!("{} [meta]", e.name)
            } else if app.git_log_mode {
                format!("{} [log]", e.name)
            } else if app.file_compare_mode {
                let names: Vec<_> = app
                    .rename_selected
                    .iter()
                    .filter_map(|&i| app.entries.get(i))
                    .map(|ent| ent.name.as_str())
                    .collect();
                format!("{} [compare]", names.join(" \u{2194} "))
            } else {
                e.name.clone()
            };
            if app.preview_wrap {
                t.push_str(" [wrap]");
            }
            t
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

    let gutter_width = if app.show_line_numbers && total > 0 {
        total.to_string().len()
    } else {
        0
    };

    // In wrap mode take more lines so ratatui has content to fold into the visible area.
    let take_count = if app.preview_wrap {
        visible_height * 5
    } else {
        visible_height
    };

    let lines: Vec<Line> = if let Some(hl) = highlighted {
        hl.into_iter()
            .skip(app.preview_scroll)
            .enumerate()
            .take(take_count)
            .map(|(i, line)| {
                if app.show_line_numbers {
                    let abs_line = app.preview_scroll + i + 1;
                    let gutter = format!("{:>width$} \u{2502} ", abs_line, width = gutter_width);
                    let gutter_span = Span::styled(gutter, Style::default().fg(Color::DarkGray));
                    let mut spans = vec![gutter_span];
                    spans.extend(line.spans);
                    Line::from(spans)
                } else {
                    line
                }
            })
            .collect()
    } else {
        app.preview_lines
            .iter()
            .enumerate()
            .skip(app.preview_scroll)
            .take(take_count)
            .map(|(i, l)| {
                let content_line = if app.preview_is_diff {
                    colorize_diff_line(l)
                } else {
                    Line::from(l.as_str())
                };
                if app.show_line_numbers {
                    let gutter = format!("{:>width$} \u{2502} ", i + 1, width = gutter_width);
                    let gutter_span = Span::styled(gutter, Style::default().fg(Color::DarkGray));
                    let mut spans = vec![gutter_span];
                    spans.extend(content_line.spans);
                    Line::from(spans)
                } else {
                    content_line
                }
            })
            .collect()
    };

    // Draw main content (leave 1 col for scrollbar).
    let content_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(title, Style::default().fg(Color::DarkGray)))
        .title_bottom(
            Line::from(Span::styled(
                scroll_info,
                Style::default().fg(Color::DarkGray),
            ))
            .right_aligned(),
        );
    let para = if app.preview_wrap {
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block)
    } else {
        Paragraph::new(lines).block(block)
    };
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

/// Render the frecency jump list as a centred overlay.
fn draw_frecency_overlay(f: &mut Frame, app: &App, size: Rect) {
    let max_visible: usize = 12;
    let row_count = app.frecency_filtered.len().max(1).min(max_visible);
    let width = 62u16.min(size.width.saturating_sub(4));
    let height = (row_count as u16 + 4)
        .min(size.height.saturating_sub(4))
        .max(6);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let title = if app.frecency_query.is_empty() {
        " Frecency ".to_string()
    } else {
        format!(" Frecency  {} ", app.frecency_query)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height.saturating_sub(1) as usize; // -1 for hint row
    let name_col = 16usize;

    if app.frecency_filtered.is_empty() {
        let msg = if app.frecency_list.is_empty() {
            "  Navigate to a directory to start tracking"
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

    let scroll = if app.frecency_selected >= visible_height {
        app.frecency_selected - visible_height + 1
    } else {
        0
    };

    let home = std::env::var("HOME").unwrap_or_default();
    let path_width = (inner.width as usize).saturating_sub(name_col + 2);

    let items: Vec<ListItem> = app
        .frecency_filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(display_idx, &real_idx)| {
            let entry = &app.frecency_list[real_idx];
            let is_selected = display_idx + scroll == app.frecency_selected;

            let short = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry.path.to_string_lossy().into_owned());
            let short_col = truncate_with_ellipsis(&short, name_col);

            let full = entry.path.to_string_lossy().into_owned();
            let display_path = if !home.is_empty() && full.starts_with(&home) {
                format!("~{}", &full[home.len()..])
            } else {
                full
            };
            let path_col = truncate_with_ellipsis(&display_path, path_width);

            if is_selected {
                let style = Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(path_col, style),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<width$}", short_col, width = name_col),
                        Style::default().fg(Color::Yellow),
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
            "Esc",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ": close  type to filter",
            Style::default().fg(Color::DarkGray),
        ),
    ]));

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

/// Render the command palette as a centered overlay.
fn draw_palette_overlay(f: &mut Frame, app: &App, size: Rect) {
    use crate::app::palette::PALETTE_ACTIONS;

    // Up to 12 rows visible; minimum 6 for empty state.
    const MAX_VISIBLE: usize = 12;
    let visible_rows = app.palette_filtered.len().clamp(1, MAX_VISIBLE) as u16;
    // +4 for border (2) + search bar (1) + footer hint (1)
    let height = (visible_rows + 4).min(size.height.saturating_sub(4)).max(6);
    let width = 72u16.min(size.width.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let title = if app.palette_query.is_empty() {
        " Command Palette ".to_string()
    } else {
        format!(" Command Palette  {} ", app.palette_query)
    };

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

    if inner.height < 2 {
        return;
    }

    // Split inner area: search bar on top, results below, hint at bottom.
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1), // search bar
            ratatui::layout::Constraint::Min(1),    // results
            ratatui::layout::Constraint::Length(1), // hint
        ])
        .split(inner);

    let search_area = chunks[0];
    let results_area = chunks[1];
    let hint_area = chunks[2];

    // Search bar
    let search_line = Line::from(vec![
        Span::styled(
            " > ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.palette_query.as_str(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(search_line), search_area);

    // Results
    if app.palette_filtered.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No matching actions",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, results_area);
    } else {
        let visible_height = results_area.height as usize;
        let scroll = if app.palette_selected >= visible_height {
            app.palette_selected - visible_height + 1
        } else {
            0
        };

        // Reserve space for key hint (10 chars + 1 space) on the right.
        let keys_width: usize = 10;
        let name_width = (results_area.width as usize).saturating_sub(keys_width + 3);

        let items: Vec<ListItem> = app
            .palette_filtered
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(display_idx, &real_idx)| {
                let action = &PALETTE_ACTIONS[real_idx];
                let is_selected = display_idx == app.palette_selected;
                let name = truncate_with_ellipsis(action.name, name_width);
                let keys = format!("{:>width$}", action.keys, width = keys_width);

                if is_selected {
                    let style = Style::default()
                        .fg(Color::White)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD);
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" \u{25cf} {:<width$} ", name, width = name_width),
                            style,
                        ),
                        Span::styled(keys, style),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("   {:<width$} ", name, width = name_width),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(keys, Style::default().fg(Color::DarkGray)),
                    ]))
                }
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, results_area);
    }

    // Footer hint
    let hint = Paragraph::new(Line::from(Span::styled(
        "  Enter=run  Esc=cancel  j/k=navigate",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(hint, hint_area);
}

fn draw_clipboard_inspect_overlay(f: &mut Frame, app: &App, size: Rect) {
    let (op_label, border_color) = match app.clipboard.as_ref().map(|c| c.op) {
        Some(ClipboardOp::Copy) => (" Clipboard — copy ", Color::Green),
        Some(ClipboardOp::Cut) => (" Clipboard — cut ", Color::Yellow),
        None => (" Clipboard — empty ", Color::DarkGray),
    };

    let truncate = |s: String| -> String {
        let max = 52usize;
        if s.chars().count() > max {
            format!("{}…", s.chars().take(max - 1).collect::<String>())
        } else {
            s
        }
    };

    let rows: Vec<Line> = if let Some(clip) = &app.clipboard {
        clip.paths
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.to_string_lossy().into_owned());
                Line::from(truncate(name))
            })
            .collect()
    } else {
        vec![Line::from(Span::styled(
            " (nothing in clipboard)",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let hint = Line::from(vec![
        Span::styled(
            " p",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" paste  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" close"),
    ]);

    let content_height = rows.len() as u16 + 1; // items + hint
    let height = (content_height + 2).min(size.height.saturating_sub(2)); // +2 border
    let width = 58u16.min(size.width);
    let x = 0;
    let y = size.height.saturating_sub(height + 1); // +1 for status bar

    let area = Rect {
        x,
        y,
        width,
        height,
    };
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(op_label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner area: items on top, hint on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(rows), chunks[0]);
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_yank_picker(f: &mut Frame, app: &App, size: Rect) {
    let Some(entry) = app.entries.get(app.selected) else {
        return;
    };

    let rel = {
        let r = entry.path.strip_prefix(&app.cwd).unwrap_or(&entry.path);
        format!("./{}", r.display())
    };
    let abs = entry.path.to_string_lossy().into_owned();
    let fname = entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.name.clone());
    let parent = entry
        .path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string());

    let truncate = |s: String| -> String {
        if s.chars().count() > 44 {
            format!("{}…", s.chars().take(43).collect::<String>())
        } else {
            s
        }
    };

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let rows = vec![
        Line::from(vec![
            Span::styled(" r ", key_style),
            Span::raw(format!(" {}", truncate(rel))),
        ]),
        Line::from(vec![
            Span::styled(" a ", key_style),
            Span::raw(format!(" {}", truncate(abs))),
        ]),
        Line::from(vec![
            Span::styled(" f ", key_style),
            Span::raw(format!(" {}", truncate(fname))),
        ]),
        Line::from(vec![
            Span::styled(" p ", key_style),
            Span::raw(format!(" {}", truncate(parent))),
        ]),
    ];

    let width = 52u16.min(size.width);
    let height = 6u16; // border top + 4 rows + border bottom
    let x = 0;
    let y = size.height.saturating_sub(height + 1); // +1 for status bar row

    let area = Rect {
        x,
        y,
        width,
        height,
    };
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Yank path ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(rows), inner);
}

fn draw_help_overlay(f: &mut Frame, size: Rect) {
    let width = 60u16.min(size.width.saturating_sub(4));
    let height = 100u16.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let help_lines = vec![
        // ── Navigation ──────────────────────────────────────────────────────
        section_header("Navigation"),
        key_line("j/Down", "Move down"),
        key_line("k/Up", "Move up"),
        key_line("l/Right", "Enter dir / open file in new cmux tab"),
        key_line("h/Left", "Go to parent"),
        key_line("Enter", "Enter dir / open file in new cmux tab"),
        key_line("g / G", "Go to top / bottom"),
        key_line("~", "Go to home directory"),
        key_line(".", "Toggle hidden files"),
        key_line("e", "Jump to path (Tab to complete, Enter to go)"),
        key_line("[ / ]", "Scroll preview pane up / down (5 lines)"),
        key_line("Ctrl+O", "Go back in directory history"),
        key_line("Ctrl+I", "Go forward in directory history"),
        key_line(
            "`<c>",
            "Set mark 'c' — record current dir to slot c (a-z A-Z)",
        ),
        key_line(
            "'<c>",
            "Jump to mark 'c' — navigate to the marked directory",
        ),
        Line::from(""),
        // ── Search ──────────────────────────────────────────────────────────
        section_header("Search"),
        key_line("/", "Fuzzy search"),
        key_line("|", "Filter/narrow listing (case-insensitive)"),
        key_line("Ctrl+F", "Content search (ripgrep)"),
        key_line("Ctrl+P", "Recursive filename find"),
        key_line("b", "Bookmark current directory"),
        key_line("B", "Open bookmark picker"),
        key_line("z", "Open frecency jump list (auto-ranked recent dirs)"),
        Line::from(""),
        // ── View ────────────────────────────────────────────────────────────
        section_header("View"),
        key_line("#", "Toggle line numbers in preview pane"),
        key_line("i", "Toggle gitignore filter (hide ignored files)"),
        key_line("d", "Toggle git diff preview"),
        key_line("V", "Toggle git log preview (commit history)"),
        key_line("D", "Toggle disk usage breakdown for directory"),
        key_line(
            "I",
            "Watch mode (auto-refresh listing on filesystem changes)",
        ),
        key_line("f", "Compare two selected files (unified diff)"),
        key_line("m", "Toggle file metadata view"),
        key_line("H", "Toggle hash preview (SHA-256 checksum)"),
        key_line("a", "Toggle hex dump view (binary inspection)"),
        key_line("w", "Toggle preview pane (hide/show)"),
        key_line("T", "Toggle timestamps / sizes in listing"),
        key_line("U", "Toggle preview word wrap"),
        key_line("N", "Toggle directory item counts"),
        key_line("P", "Edit file permissions (chmod)"),
        key_line("R", "Refresh git status"),
        key_line("S", "Cycle sort: Name/Size/Modified/Ext"),
        key_line("s", "Toggle sort order ↑↓"),
        Line::from(""),
        // ── Selection & Rename ──────────────────────────────────────────────
        section_header("Selection & Rename"),
        key_line("J / K", "Extend selection down / up (range select)"),
        key_line("Space", "Toggle file selection"),
        key_line("v", "Select all files"),
        key_line("*", "Select files by glob pattern (e.g. *.rs)"),
        key_line("n / F2", "Quick rename (inline bar pre-filled)"),
        key_line("r", "Bulk rename selected files with regex"),
        key_line("Esc", "Clear filter (if active) or selections"),
        Line::from(""),
        // ── File Operations ─────────────────────────────────────────────────
        section_header("File Operations"),
        key_line("o", "Open in $EDITOR (suspends TUI)"),
        key_line("O", "Open with system default (background)"),
        key_line("c / C", "Copy current / selected"),
        key_line("x", "Cut current to clipboard"),
        key_line("F", "Inspect clipboard contents"),
        key_line("p", "Paste clipboard into current dir"),
        key_line("Delete / X", "Trash current / selected (recoverable)"),
        key_line("u", "Undo last trash operation"),
        key_line("t", "New file (touch — create empty file)"),
        key_line("W", "Duplicate selected entry in place"),
        key_line("L", "Create symlink to selected entry"),
        key_line("Z", "Extract archive to current directory"),
        key_line("E", "Create archive from selected files (tar.gz, zip, …)"),
        key_line("M", "Make new directory"),
        Line::from(""),
        // ── Yank & Misc ─────────────────────────────────────────────────────
        section_header("Yank & Misc"),
        key_line("y / Y", "Yank relative / absolute path"),
        key_line(
            "A",
            "Yank path (pick format: r=relative a=absolute f=filename p=parent)",
        ),
        key_line(":", "Open command palette"),
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
