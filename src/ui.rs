use crate::app::session_snapshot::ChangeKind;
use crate::app::session_summary::count_by_kind;
use crate::app::{format_dir_count, format_listing_date, format_tokens, App, SortMode, SortOrder};
use crate::git::FileStatus;
use crate::icons::icon_for_entry;
use crate::ops::ClipboardOp;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Fill the entire frame with the theme's background colour.
///
/// This is a no-op when `theme.bg` is `Color::Reset` (all built-in themes
/// except those that own their surface colour, like `norton-commander`).
/// For themes that set an explicit `bg`, this call ensures that any area
/// ratatui hasn't painted with a widget still shows the correct colour
/// instead of the terminal's default background.
fn paint_background(f: &mut Frame, theme: &crate::theme::Theme) {
    if theme.bg == Color::Reset {
        return;
    }
    let style = Style::default().bg(theme.bg);
    f.render_widget(Block::default().style(style), f.size());
}

/// Main draw function. Computes pane layout from app's divider fractions,
/// then renders parent pane, current-dir pane, and preview pane.
pub fn draw(f: &mut Frame, app: &mut App) {
    paint_background(f, &app.theme);

    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // path bar
            Constraint::Min(3),    // main panes
            Constraint::Length(1), // status bar
        ])
        .split(size);

    let path_area = chunks[0];
    let main_area = chunks[1];
    let bottom_area = chunks[2];

    // Draw path bar.
    draw_path_bar(f, app, path_area);

    // Compute column positions of the two dividers.
    let left_cols = if app.left_collapsed {
        0
    } else {
        ((app.left_div * main_area.width as f64).round() as u16).max(3)
    };
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
    if app.overlay.session_summary_mode {
        draw_session_summary_pane(f, app, pane_chunks[1]);
    } else if app.overlay.content_search_mode {
        draw_content_search_pane(f, app, pane_chunks[1]);
    } else if app.overlay.find_mode {
        draw_find_pane(f, app, pane_chunks[1]);
    } else {
        draw_current_pane(f, app, pane_chunks[1]);
    }
    if !app.preview.preview_collapsed {
        if app.overlay.change_feed_mode {
            draw_change_feed_pane(f, app, pane_chunks[2]);
        } else if app.overlay.task_manager_mode {
            draw_task_manager_pane(f, app, pane_chunks[2]);
        } else {
            draw_preview_pane(f, app, pane_chunks[2]);
        }
    }

    // Draw bottom bar.
    if !app.pending_delete.is_empty() {
        draw_delete_confirm_bar(f, app, bottom_area);
    } else if let Some(ref path) = app.pending_extract {
        draw_extract_bar(f, app, bottom_area, path);
    } else if app.overlay.quick_rename_mode {
        draw_quick_rename_bar(f, app, bottom_area);
    } else if app.overlay.path_mode {
        draw_path_jump_bar(f, app, bottom_area);
    } else if app.overlay.dup_mode {
        draw_dup_bar(f, app, bottom_area);
    } else if app.overlay.symlink_mode {
        draw_symlink_bar(f, app, bottom_area);
    } else if app.overlay.mkdir_mode {
        draw_mkdir_bar(f, app, bottom_area);
    } else if app.overlay.touch_mode {
        draw_touch_bar(f, app, bottom_area);
    } else if app.overlay.chmod_mode {
        draw_chmod_bar(f, app, bottom_area);
    } else if app.overlay.content_search_mode {
        draw_content_search_bar(f, app, bottom_area);
    } else if app.overlay.find_mode {
        draw_find_bar(f, app, bottom_area);
    } else if app.nav.filter_mode {
        draw_filter_bar(f, app, bottom_area);
    } else if app.nav.search_mode {
        draw_search_bar(f, app, bottom_area);
    } else if app.overlay.session_summary_mode {
        let para = Paragraph::new(Line::from(vec![
            Span::styled(
                " [session summary] ",
                Style::default()
                    .fg(app.theme.info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "j/k: navigate  l/Enter: go to file  C: reset checkpoint  R: refresh  Esc: exit",
                Style::default().fg(app.theme.fg_dim),
            ),
        ]));
        f.render_widget(para, bottom_area);
    } else if let Some(ref msg) = app.status_message {
        let para = Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default()
                .fg(app.theme.ok)
                .add_modifier(Modifier::BOLD),
        )));
        f.render_widget(para, bottom_area);
    } else if !app.nav.selection.is_empty() {
        let count = app.nav.selection.len();
        let total_bytes: u64 = app
            .nav
            .selection
            .iter()
            .filter_map(|&i| app.nav.entries.get(i))
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        let size_label = if total_bytes > 0 {
            format!("  ({})", format_tokens(total_bytes))
        } else {
            String::new()
        };
        let para = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} selected{}", count, size_label),
                Style::default()
                    .fg(app.theme.multi_sel_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  — v: all   Esc: clear",
                Style::default().fg(app.theme.fg_dim),
            ),
        ]));
        f.render_widget(para, bottom_area);
    } else if let Some(ref clip) = app.clipboard {
        // Show clipboard indicator.
        let (label, color) = match clip.op {
            ClipboardOp::Copy => ("[copy]", app.theme.ok),
            ClipboardOp::Cut => ("[cut]", app.theme.warn),
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
                Style::default().fg(app.theme.fg_dim),
            ),
        ]));
        f.render_widget(para, bottom_area);
    } else {
        // Show hint.
        let hint = Paragraph::new(Line::from(Span::styled(
            " Press ? for help",
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(hint, bottom_area);
    }

    // Help overlay.
    if app.overlay.show_help {
        draw_help_overlay(f, app, size);
    }

    // Bookmark picker overlay.
    if app.overlay.bookmark_mode {
        draw_bookmark_overlay(f, app, size);
    }

    // Frecency jump overlay.
    if app.overlay.frecency_mode {
        draw_frecency_overlay(f, app, size);
    }

    // Yank picker overlay.
    if app.overlay.yank_picker_mode {
        draw_yank_picker(f, app, size);
    }

    // Context bundle picker overlay.
    if app.overlay.context_bundle_picker_mode {
        draw_context_bundle_picker(f, app, size);
    }

    // cmux surface picker overlay (send selected lines to a surface).
    if app.overlay.cmux_surface_picker_mode {
        draw_cmux_surface_picker(f, app, size);
    }

    // Clipboard inspector overlay.
    if app.overlay.clipboard_inspect_mode {
        draw_clipboard_inspect_overlay(f, app, size);
    }

    // Command palette overlay (rendered on top of everything else).
    if app.overlay.palette_mode {
        draw_palette_overlay(f, app, size);
    }
}

fn draw_path_bar(f: &mut Frame, app: &App, area: Rect) {
    // In archive mode show the virtual breadcrumb instead of the real cwd.
    if app.overlay.archive_mode {
        let crumb = app.archive_breadcrumb();
        let spans = vec![
            Span::styled(
                " \u{1f4e6} ",
                Style::default()
                    .fg(app.theme.warn)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                crumb,
                Style::default()
                    .fg(app.theme.fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  [archive]  Esc: exit  h: up  l: enter",
                Style::default().fg(app.theme.fg_dim),
            ),
        ];
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    // Smart path truncation: keep last 3 components when path is wide.
    let available = area.width.saturating_sub(4) as usize; // rough margin
    let path_str = app.nav.cwd.to_string_lossy();
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
            .fg(app.theme.fg)
            .add_modifier(Modifier::BOLD),
    )];

    // Hidden files indicator as a separate, dimmed span.
    if app.nav.show_hidden {
        spans.push(Span::styled("  [H]", Style::default().fg(app.theme.fg_dim)));
    }

    // Gitignore filter badge — shown next to the git branch indicator.
    if app.nav.hide_gitignored {
        spans.push(Span::styled(
            "  [ignore]",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(branch) = app.git_status.as_ref().and_then(|g| g.branch.as_ref()) {
        spans.push(Span::styled(
            format!("  ({})", branch),
            Style::default()
                .fg(app.theme.ok)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Show sort indicator when not using the default (Name ascending).
    if app.nav.sort_mode != SortMode::Name || app.nav.sort_order != SortOrder::Ascending {
        let arrow = if app.nav.sort_order == SortOrder::Descending {
            "↓"
        } else {
            "↑"
        };
        spans.push(Span::styled(
            format!("  {} {}", arrow, app.nav.sort_mode.label()),
            Style::default().fg(app.theme.fg_dim),
        ));
    }

    // Watcher indicator — shown when the filesystem watcher is active.
    if app.watcher.is_some() {
        spans.push(Span::styled(
            "  [watch]",
            Style::default()
                .fg(app.theme.info)
                .add_modifier(Modifier::BOLD),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let match_count = app.nav.filtered_indices.len();
    let total = app.nav.entries.len();
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "/",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.nav.search_query,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "\u{2588}", // block cursor
            Style::default().fg(app.theme.cursor),
        ),
        Span::styled(
            format!(" [{}/{}]", match_count, total),
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Filter: ",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}_", app.nav.filter_input),
            Style::default().fg(app.theme.input),
        ),
        Span::styled(
            "  Esc=clear  Enter=freeze",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(para, area);
}

fn draw_extract_bar(f: &mut Frame, app: &App, area: Rect, path: &std::path::Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Extract ",
            Style::default()
                .fg(app.theme.confirm_fg)
                .bg(app.theme.confirm_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  \"{}\" → ./   ", name)),
        Span::styled("[y/Enter]", Style::default().fg(app.theme.ok)),
        Span::raw("confirm  "),
        Span::styled("[Esc]", Style::default().fg(app.theme.fg_dim)),
        Span::styled("cancel", Style::default().fg(app.theme.fg_dim)),
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
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[t/y]", Style::default().fg(app.theme.ok)),
        Span::styled("trash  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled("[D]", Style::default().fg(app.theme.error)),
        Span::styled(
            "delete permanently  ",
            Style::default().fg(app.theme.fg_dim),
        ),
        Span::styled("[Esc]", Style::default().fg(app.theme.fg_dim)),
        Span::styled("cancel", Style::default().fg(app.theme.fg_dim)),
    ]));
    f.render_widget(para, area);
}

fn draw_chmod_bar(f: &mut Frame, app: &App, area: Rect) {
    // Show the current octal mode as context.
    let current = app
        .nav
        .entries
        .get(app.nav.selected)
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
        .nav
        .entries
        .get(app.nav.selected)
        .map(|e| e.name.as_str())
        .unwrap_or("");

    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" chmod {} [current: {}]: ", name, current),
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.overlay.chmod_input.as_str(),
            Style::default().fg(app.theme.input),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter=apply  Esc=cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_quick_rename_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Rename: ",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.overlay.quick_rename_input.as_str(),
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter=confirm  Esc=cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_path_jump_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            " Jump to: ",
            Style::default()
                .fg(app.theme.prompt_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.overlay.path_input.as_str(),
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Tab=complete  Enter=go  Esc=cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_mkdir_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "New directory: ",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.overlay.mkdir_input,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter: create   Esc: cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_touch_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "New file: ",
            Style::default()
                .fg(app.theme.prompt_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.overlay.touch_input,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter: create   Esc: cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_dup_bar(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Duplicate: ",
            Style::default()
                .fg(app.theme.prompt_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.overlay.dup_input,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter: copy   Esc: cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_symlink_bar(f: &mut Frame, app: &App, area: Rect) {
    let target_name = app
        .overlay
        .symlink_target
        .as_deref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "\u{2026}".to_string());
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("Symlink \u{2192} {} : ", target_name),
            Style::default()
                .fg(app.theme.prompt_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.overlay.symlink_input,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter: create   Esc: cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

fn draw_parent_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .nav
        .cwd
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string());

    let inner_width = area.width.saturating_sub(2) as usize; // account for right border
    let visible_height = area.height.saturating_sub(1) as usize;
    let items: Vec<ListItem> = app
        .nav
        .parent_entries
        .iter()
        .enumerate()
        .skip(app.nav.parent_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let style = if i == app.nav.parent_selected {
                Style::default()
                    .fg(app.theme.sel_fg)
                    .bg(app.theme.sel_bg)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default()
                    .fg(app.theme.dir_fg)
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
            .border_style(Style::default().fg(app.theme.border))
            .title(Span::styled(title, Style::default().fg(app.theme.border))),
    );
    f.render_widget(list, area);
}

fn draw_current_pane(f: &mut Frame, app: &App, area: Rect) {
    let base_title = app
        .nav
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.nav.cwd.to_string_lossy().into_owned());

    // Show [~pattern] when filter is frozen (active but bar is closed).
    let title = if !app.nav.filter_input.is_empty() && !app.nav.filter_mode {
        format!("{} [~{}]", base_title, app.nav.filter_input)
    } else {
        base_title
    };

    let is_searching = app.nav.search_mode && !app.nav.search_query.is_empty();
    // 2-char prefix always reserved so layout doesn't shift when selection changes.
    let sel_prefix_width: usize = 2;
    let has_selection = !app.nav.selection.is_empty();

    let inner_width = area.width.saturating_sub(1) as usize; // 1 col for right border
    let visible_height = area.height.saturating_sub(2) as usize; // top title + bottom info
    let items: Vec<ListItem> = app
        .nav
        .entries
        .iter()
        .enumerate()
        .skip(app.nav.current_scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let is_cursor = i == app.nav.selected;
            let is_marked = app.nav.selection.contains(&i);
            let is_match = !is_searching || app.nav.filtered_set.contains(&i);
            let style = if is_cursor {
                Style::default()
                    .fg(app.theme.sel_fg)
                    .bg(app.theme.sel_bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_marked {
                Style::default()
                    .fg(app.theme.multi_sel_fg)
                    .add_modifier(Modifier::BOLD)
            } else if !is_match {
                Style::default().fg(app.theme.fg_dim)
            } else if entry.is_dir {
                Style::default()
                    .fg(app.theme.dir_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Determine git status indicator (char, color) for this entry.
            let git_indicator: Option<(char, Color)> = app.git_status.as_ref().and_then(|git| {
                if entry.is_dir {
                    if git.subtree_dirty(&entry.path) {
                        Some(('\u{25cf}', app.theme.git_modified)) // ● dimmed for dirty dir
                    } else {
                        None
                    }
                } else {
                    git.for_path(&entry.path)
                        .map(|s| file_status_indicator(s, &app.theme))
                }
            });

            let icon = icon_for_entry(&entry.name, entry.is_dir);
            // Right column priority: timestamps > dir counts > file size.
            let right_col_str: String = if app.nav.show_timestamps {
                if entry.is_dir {
                    String::new()
                } else {
                    format_listing_date(entry.modified)
                }
            } else if entry.is_dir && app.nav.show_dir_counts {
                format_dir_count(entry.child_count)
            } else if entry.is_dir {
                String::new()
            } else {
                format_tokens(entry.size)
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
                        .fg(app.theme.multi_sel_mark)
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
                        .bg(app.theme.sel_bg)
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
                    Style::default().fg(app.theme.fg_dim).bg(app.theme.sel_bg)
                } else {
                    Style::default().fg(app.theme.fg_dim)
                };
                spans.push(Span::styled(right_col_str, col_style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let info = if app.nav.entries_truncated {
        format!(
            " {}/{} [limit] ",
            app.nav.selected + 1,
            app.nav.entries.len()
        )
    } else {
        format!(" {}/{} ", app.nav.selected + 1, app.nav.entries.len())
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP | Borders::RIGHT)
            .border_style(Style::default().fg(app.theme.border_focus))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(app.theme.fg)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_bottom(
                Line::from(Span::styled(info, Style::default().fg(app.theme.fg_dim)))
                    .right_aligned(),
            ),
    );
    f.render_widget(list, area);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .nav
        .entries
        .get(app.nav.selected)
        .map(|e| {
            let mut t = if app.preview.du_preview_mode && e.is_dir {
                format!("{} [du]", e.name)
            } else if app.preview.hex_view_mode {
                format!("{} [hex]", e.name)
            } else if app.preview.preview_is_diff {
                format!("{} [diff]", e.name)
            } else if app.preview.meta_preview_mode {
                format!("{} [meta]", e.name)
            } else if app.preview.git_log_mode {
                format!("{} [log]", e.name)
            } else if app.preview.file_compare_mode {
                let names: Vec<_> = app
                    .nav
                    .selection
                    .iter()
                    .filter_map(|&i| app.nav.entries.get(i))
                    .map(|ent| ent.name.as_str())
                    .collect();
                format!("{} [compare]", names.join(" \u{2194} "))
            } else {
                e.name.clone()
            };
            if app.preview.preview_wrap {
                t.push_str(" [wrap]");
            }
            t
        })
        .unwrap_or_default();

    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.preview.preview_lines.len();
    let scroll_info = if total > 0 {
        let end = (app.preview.preview_scroll + visible_height).min(total);
        format!(" {}-{}/{} ", app.preview.preview_scroll + 1, end, total)
    } else {
        String::new()
    };

    // Try syntax highlighting for source files (non-diff mode only).
    let highlighted: Option<Vec<Line<'static>>> = if !app.preview.preview_is_diff {
        app.nav.entries.get(app.nav.selected).and_then(|e| {
            let ext = std::path::Path::new(&e.name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext.is_empty() {
                None
            } else {
                let max_process = (app.preview.preview_scroll + visible_height)
                    .min(app.preview.preview_lines.len());
                app.highlighter.highlight(
                    &app.preview.preview_lines[..max_process],
                    ext,
                    max_process,
                    app.theme.syntax_theme,
                )
            }
        })
    } else {
        None
    };

    let gutter_width = if app.preview.show_line_numbers && total > 0 {
        total.to_string().len()
    } else {
        0
    };

    // In wrap mode take more lines so ratatui has content to fold into the visible area.
    let take_count = if app.preview.preview_wrap {
        visible_height * 5
    } else {
        visible_height
    };

    // Compute cursor/selection range for preview focus mode.
    let selection_range: Option<(usize, usize)> = if app.preview.preview_focused {
        app.preview.preview_selection_anchor.map(|anchor| {
            let lo = anchor.min(app.preview.preview_cursor);
            let hi = anchor.max(app.preview.preview_cursor);
            (lo, hi)
        })
    } else {
        None
    };

    let lines: Vec<Line> = if let Some(hl) = highlighted {
        hl.into_iter()
            .skip(app.preview.preview_scroll)
            .enumerate()
            .take(take_count)
            .map(|(i, line)| {
                let abs_line = app.preview.preview_scroll + i;
                let is_cursor =
                    app.preview.preview_focused && abs_line == app.preview.preview_cursor;
                let in_selection =
                    selection_range.is_some_and(|(lo, hi)| abs_line >= lo && abs_line <= hi);
                let row_style = if is_cursor {
                    Style::default()
                        .bg(app.theme.sel_bg)
                        .fg(app.theme.sel_fg)
                        .add_modifier(Modifier::BOLD)
                } else if in_selection {
                    Style::default().bg(app.theme.subtle_sel_bg)
                } else {
                    Style::default()
                };
                let rendered = if app.preview.show_line_numbers {
                    let gutter =
                        format!("{:>width$} \u{2502} ", abs_line + 1, width = gutter_width);
                    let gutter_span = Span::styled(gutter, Style::default().fg(app.theme.fg_dim));
                    let mut spans = vec![gutter_span];
                    spans.extend(line.spans);
                    Line::from(spans)
                } else {
                    line
                };
                rendered.patch_style(row_style)
            })
            .collect()
    } else {
        app.preview
            .preview_lines
            .iter()
            .enumerate()
            .skip(app.preview.preview_scroll)
            .take(take_count)
            .map(|(i, l)| {
                let abs_line = i; // `i` is the absolute index since we enumerate before skip
                let is_cursor =
                    app.preview.preview_focused && abs_line == app.preview.preview_cursor;
                let in_selection =
                    selection_range.is_some_and(|(lo, hi)| abs_line >= lo && abs_line <= hi);
                let row_style = if is_cursor {
                    Style::default()
                        .bg(app.theme.sel_bg)
                        .fg(app.theme.sel_fg)
                        .add_modifier(Modifier::BOLD)
                } else if in_selection {
                    Style::default().bg(app.theme.subtle_sel_bg)
                } else {
                    Style::default()
                };
                let content_line = if app.preview.preview_is_diff {
                    colorize_diff_line(l, &app.theme)
                } else {
                    Line::from(l.as_str())
                };
                let rendered = if app.preview.show_line_numbers {
                    let gutter = format!("{:>width$} \u{2502} ", i + 1, width = gutter_width);
                    let gutter_span = Span::styled(gutter, Style::default().fg(app.theme.fg_dim));
                    let mut spans = vec![gutter_span];
                    spans.extend(content_line.spans);
                    Line::from(spans)
                } else {
                    content_line
                };
                rendered.patch_style(row_style)
            })
            .collect()
    };

    // Border/title color changes to Cyan when preview pane has focus.
    let border_color = if app.preview.preview_focused {
        app.theme.border_focus
    } else {
        app.theme.border
    };

    // Draw main content (leave 1 col for scrollbar).
    let content_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(border_color)))
        .title_bottom(
            Line::from(Span::styled(scroll_info, Style::default().fg(border_color)))
                .right_aligned(),
        );
    // Show a loading placeholder while the async preview thread is working.
    if app.preview.preview_loading && app.preview.preview_lines.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            " Loading\u{2026}",
            Style::default().fg(app.theme.fg_dim),
        )))
        .block(block);
        f.render_widget(placeholder, content_area);
        return;
    }

    let para = if app.preview.preview_wrap {
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
                ((app.preview.preview_scroll as f64 / max_scroll as f64) * max_thumb_pos as f64)
                    .round() as usize
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
                    Style::default().fg(app.theme.scrollbar_thumb)
                } else {
                    Style::default().fg(app.theme.scrollbar_track)
                };
                f.render_widget(Paragraph::new(Line::from(Span::styled(ch, style))), r);
            }
        }
    }
}

/// Render the live change feed in the preview pane area when change_feed_mode is on.
fn draw_change_feed_pane(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::change_feed::FeedEventKind;

    let paused = app.recursive_watcher.is_none();
    let title = if paused {
        " Change Feed (paused) "
    } else {
        " Change Feed "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.change_feed.events.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No changes recorded yet",
            Style::default().fg(app.theme.fg_dim),
        )))
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    let now = std::time::Instant::now();
    let height = inner.height as usize;
    let total = app.change_feed.events.len();
    let selected = app.change_feed.selected;

    // Compute a scroll window that keeps `selected` visible.
    let scroll_offset = if selected < height {
        0
    } else {
        selected - height + 1
    };

    let items: Vec<ListItem> = app
        .change_feed
        .events
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(height)
        .map(|(idx, ev)| {
            let elapsed = now.duration_since(ev.recorded_at);
            let secs = elapsed.as_secs();
            let age = format!("{:02}:{:02}", secs / 60, secs % 60);

            // Compute path relative to the feed root.
            let display_path = ev
                .path
                .strip_prefix(&app.change_feed_root)
                .unwrap_or(&ev.path)
                .to_string_lossy()
                .into_owned();

            // Left-truncate to fit the pane width.
            let max_path_chars = inner.width.saturating_sub(12) as usize;
            let display_path =
                if display_path.chars().count() > max_path_chars && max_path_chars > 3 {
                    let skip = display_path.chars().count() - max_path_chars + 1;
                    format!("…{}", &display_path[skip..])
                } else {
                    display_path
                };

            let sym = ev.kind.symbol();
            let sym_color = match ev.kind {
                FeedEventKind::Created => app.theme.event_new,
                FeedEventKind::Modified => app.theme.event_modified,
                FeedEventKind::Deleted => app.theme.event_deleted,
            };

            let is_selected = idx == selected;
            let base_style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {:>5}  ", age), base_style.fg(app.theme.fg_dim)),
                Span::styled(
                    format!("{} ", sym),
                    if is_selected {
                        base_style.fg(sym_color)
                    } else {
                        Style::default().fg(sym_color)
                    },
                ),
                Span::styled(display_path, base_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);

    // Scrollbar indicator when there are more events than visible rows.
    if total > height {
        let indicator = format!(" {}/{} ", selected + 1, total);
        let ind_len = indicator.len() as u16;
        if inner.width > ind_len + 2 {
            let ind_area = Rect::new(
                inner.x + inner.width - ind_len,
                inner.y + inner.height.saturating_sub(1),
                ind_len,
                1,
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    indicator,
                    Style::default().fg(app.theme.fg_dim),
                )),
                ind_area,
            );
        }
    }
}

/// Render grouped rg results in the center pane during content search mode.
fn draw_content_search_pane(f: &mut Frame, app: &App, area: Rect) {
    let total_matches: usize = app
        .overlay
        .content_search_results
        .iter()
        .map(|g| g.matches.len())
        .sum();
    let file_count = app.overlay.content_search_results.len();

    let title = if total_matches > 0 {
        let trunc = if app.overlay.content_search_truncated {
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
    } else if app.overlay.content_search_query.is_empty() {
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
    for group in &app.overlay.content_search_results {
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
        .position(|r| matches!(r, RowKind::Match { flat_idx: fi, .. } if *fi == app.overlay.content_search_selected))
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
                    .fg(app.theme.info)
                    .add_modifier(Modifier::BOLD),
            ))),
            RowKind::Match {
                flat_idx: fi,
                line_number,
                content,
            } => {
                let is_selected = *fi == app.overlay.content_search_selected;
                let style = if is_selected {
                    Style::default()
                        .fg(app.theme.sel_fg)
                        .bg(app.theme.sel_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let num_style = if is_selected {
                    style
                } else {
                    Style::default().fg(app.theme.fg_dim)
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
            .border_style(Style::default().fg(app.theme.border))
            .title(title),
    );
    f.render_widget(list, area);
}

/// Render the content search prompt in the status bar.
fn draw_content_search_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref err) = app.overlay.content_search_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!(" \u{26a0} {}", err),
            Style::default().fg(app.theme.error),
        )));
        f.render_widget(para, area);
        return;
    }
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Search contents: ",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.overlay.content_search_query,
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
        Span::styled(
            "  Enter: run   Esc: cancel",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(para, area);
}

/// Map a `FileStatus` to a display character and colour.
fn file_status_indicator(status: FileStatus, theme: &crate::theme::Theme) -> (char, Color) {
    match status {
        FileStatus::Conflict => ('\u{2716}', theme.git_deleted), // ✖
        FileStatus::Deleted => ('\u{2716}', theme.git_deleted),  // ✖
        FileStatus::Staged | FileStatus::StagedModified => ('\u{271a}', theme.git_staged), // ✚
        FileStatus::Modified => ('\u{25cf}', theme.git_modified), // ●
        FileStatus::Untracked => ('+', theme.git_untracked),
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
fn colorize_diff_line<'a>(line: &'a str, theme: &crate::theme::Theme) -> Line<'a> {
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(theme.diff_add)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(theme.diff_del)
    } else if line.starts_with("@@") {
        Style::default().fg(theme.diff_hunk)
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        Style::default().fg(theme.diff_meta)
    } else {
        Style::default()
    };
    Line::from(Span::styled(line, style))
}

fn section_header(label: &'static str, theme: &crate::theme::Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", label),
        Style::default()
            .fg(theme.prompt)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_line(key: &'static str, desc: &'static str, theme: &crate::theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<10}", key), Style::default().fg(theme.info)),
        Span::raw(desc),
    ])
}

/// Render recursive find results in the center pane during find mode.
fn draw_find_pane(f: &mut Frame, app: &App, area: Rect) {
    let count = app.overlay.find_results.len();
    let title = if app.overlay.find_query.is_empty() {
        " Find files ".to_string()
    } else if count == 0 {
        " No matches ".to_string()
    } else {
        let trunc = if app.overlay.find_truncated {
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
    let scroll = if app.overlay.find_selected >= visible_height {
        app.overlay.find_selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .overlay
        .find_results
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, r)| {
            let is_selected = i == app.overlay.find_selected;
            let style = if is_selected {
                Style::default()
                    .fg(app.theme.sel_fg)
                    .bg(app.theme.sel_bg)
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
            .border_style(Style::default().fg(app.theme.border))
            .title(title),
    );
    f.render_widget(list, area);
}

/// Render the bookmark picker as a centered overlay.
fn draw_bookmark_overlay(f: &mut Frame, app: &App, size: Rect) {
    // Compute overlay dimensions.  Minimum usable height is 6 rows.
    let width = 62u16.min(size.width.saturating_sub(4));
    let max_rows = app.overlay.bookmark_filtered.len().max(1) as u16;
    let height = (max_rows + 4).min(size.height.saturating_sub(4)).max(6);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    // Title: show filter query when active.
    let title = if app.overlay.bookmark_query.is_empty() {
        " Bookmarks ".to_string()
    } else {
        format!(" Bookmarks  {} ", app.overlay.bookmark_query)
    };

    // Hint in title right section — put it in the title for simplicity.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus))
        .title(Span::styled(
            title,
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let name_col = 16usize; // chars reserved for short name

    if app.overlay.bookmark_filtered.is_empty() {
        let msg = if app.overlay.bookmarks.is_empty() {
            "  No bookmarks — press b to add one"
        } else {
            "  No matches"
        };
        let para = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(para, inner);
        return;
    }

    let scroll = if app.overlay.bookmark_selected >= visible_height {
        app.overlay.bookmark_selected - visible_height + 1
    } else {
        0
    };

    let path_width = (inner.width as usize).saturating_sub(name_col + 2);

    let items: Vec<ListItem> = app
        .overlay
        .bookmark_filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(display_idx, &real_idx)| {
            let path = &app.overlay.bookmarks[real_idx];
            let exists = path.is_dir();
            let is_selected = display_idx == app.overlay.bookmark_selected;

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
                    .fg(app.theme.sel_fg)
                    .bg(app.theme.sel_bg)
                    .add_modifier(Modifier::BOLD);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(format!("{}{}", path_col, gone_suffix), style),
                ]))
            } else if !exists {
                let style = Style::default().fg(app.theme.fg_dim);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(format!("{}{}", path_col, gone_suffix), style),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<width$}", short_col, width = name_col),
                        Style::default().fg(app.theme.info),
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
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": jump  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            "d",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": remove  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": close", Style::default().fg(app.theme.fg_dim)),
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
    let row_count = app.overlay.frecency_filtered.len().max(1).min(max_visible);
    let width = 62u16.min(size.width.saturating_sub(4));
    let height = (row_count as u16 + 4)
        .min(size.height.saturating_sub(4))
        .max(6);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let title = if app.overlay.frecency_query.is_empty() {
        " Frecency ".to_string()
    } else {
        format!(" Frecency  {} ", app.overlay.frecency_query)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_warn))
        .title(Span::styled(
            title,
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height.saturating_sub(1) as usize; // -1 for hint row
    let name_col = 16usize;

    if app.overlay.frecency_filtered.is_empty() {
        let msg = if app.nav.frecency_list.is_empty() {
            "  Navigate to a directory to start tracking"
        } else {
            "  No matches"
        };
        let para = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(para, inner);
        return;
    }

    let scroll = if app.overlay.frecency_selected >= visible_height {
        app.overlay.frecency_selected - visible_height + 1
    } else {
        0
    };

    let home = std::env::var("HOME").unwrap_or_default();
    let path_width = (inner.width as usize).saturating_sub(name_col + 2);

    let items: Vec<ListItem> = app
        .overlay
        .frecency_filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(display_idx, &real_idx)| {
            let entry = &app.nav.frecency_list[real_idx];
            let is_selected = display_idx + scroll == app.overlay.frecency_selected;

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
                    .fg(app.theme.sel_fg)
                    .bg(app.theme.sel_bg)
                    .add_modifier(Modifier::BOLD);
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<width$}", short_col, width = name_col), style),
                    Span::styled(path_col, style),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<width$}", short_col, width = name_col),
                        Style::default().fg(app.theme.warn),
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
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": jump  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ": close  type to filter",
            Style::default().fg(app.theme.fg_dim),
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
    if let Some(ref err) = app.overlay.find_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!(" \u{26a0} {}", err),
            Style::default().fg(app.theme.error),
        )));
        f.render_widget(para, area);
        return;
    }
    let para = Paragraph::new(Line::from(vec![
        Span::styled(
            "Find: ",
            Style::default()
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.overlay.find_query.as_str(),
            Style::default().fg(app.theme.input),
        ),
        Span::styled("█", Style::default().fg(app.theme.cursor)),
    ]));
    f.render_widget(para, area);
}

/// Render the command palette as a centered overlay.
fn draw_palette_overlay(f: &mut Frame, app: &App, size: Rect) {
    use crate::app::palette::PALETTE_ACTIONS;

    // Up to 12 rows visible; minimum 6 for empty state.
    const MAX_VISIBLE: usize = 12;
    let visible_rows = app.overlay.palette_filtered.len().clamp(1, MAX_VISIBLE) as u16;
    // +4 for border (2) + search bar (1) + footer hint (1)
    let height = (visible_rows + 4).min(size.height.saturating_sub(4)).max(6);
    let width = 72u16.min(size.width.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let title = if app.overlay.palette_query.is_empty() {
        " Command Palette ".to_string()
    } else {
        format!(" Command Palette  {} ", app.overlay.palette_query)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus))
        .title(Span::styled(
            title,
            Style::default()
                .fg(app.theme.fg)
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
                .fg(app.theme.prompt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.overlay.palette_query.as_str(),
            Style::default()
                .fg(app.theme.input)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(app.theme.cursor)),
    ]);
    f.render_widget(Paragraph::new(search_line), search_area);

    // Results
    if app.overlay.palette_filtered.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No matching actions",
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(empty, results_area);
    } else {
        let visible_height = results_area.height as usize;
        let scroll = if app.overlay.palette_selected >= visible_height {
            app.overlay.palette_selected - visible_height + 1
        } else {
            0
        };

        // Reserve space for key hint (10 chars + 1 space) on the right.
        let keys_width: usize = 10;
        let name_width = (results_area.width as usize).saturating_sub(keys_width + 3);

        let items: Vec<ListItem> = app
            .overlay
            .palette_filtered
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(display_idx, &real_idx)| {
                let action = &PALETTE_ACTIONS[real_idx];
                let is_selected = display_idx == app.overlay.palette_selected;
                let name = truncate_with_ellipsis(action.name, name_width);
                let keys = format!("{:>width$}", action.keys, width = keys_width);

                if is_selected {
                    let style = Style::default()
                        .fg(app.theme.sel_fg)
                        .bg(app.theme.sel_bg)
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
                            Style::default().fg(app.theme.fg),
                        ),
                        Span::styled(keys, Style::default().fg(app.theme.fg_dim)),
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
        Style::default().fg(app.theme.fg_dim),
    )));
    f.render_widget(hint, hint_area);
}

fn draw_clipboard_inspect_overlay(f: &mut Frame, app: &App, size: Rect) {
    let (op_label, border_color) = match app.clipboard.as_ref().map(|c| c.op) {
        Some(ClipboardOp::Copy) => (" Clipboard — copy ", app.theme.ok),
        Some(ClipboardOp::Cut) => (" Clipboard — cut ", app.theme.warn),
        None => (" Clipboard — empty ", app.theme.border),
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
            Style::default().fg(app.theme.fg_dim),
        ))]
    };

    let hint = Line::from(vec![
        Span::styled(
            " p",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" paste  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(app.theme.warn)
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
    let Some(entry) = app.nav.entries.get(app.nav.selected) else {
        return;
    };

    let rel = {
        let r = entry.path.strip_prefix(&app.nav.cwd).unwrap_or(&entry.path);
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
        .fg(app.theme.warn)
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
        .border_style(Style::default().fg(app.theme.border_warn));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(rows), inner);
}

fn draw_context_bundle_picker(f: &mut Frame, app: &App, size: Rect) {
    let key_style = Style::default()
        .fg(app.theme.warn)
        .add_modifier(Modifier::BOLD);
    let rows = vec![
        Line::from(vec![
            Span::styled(" p ", key_style),
            Span::raw("  paths only"),
        ]),
        Line::from(vec![
            Span::styled(" c ", key_style),
            Span::raw("  paths + contents"),
        ]),
        Line::from(vec![
            Span::styled(" d ", key_style),
            Span::raw("  paths + diff"),
        ]),
        Line::from(vec![Span::styled(" Esc ", key_style), Span::raw(" cancel")]),
    ];

    let width = 34u16.min(size.width);
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
        .title(" Export context bundle ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(rows), inner);
}

fn draw_help_overlay(f: &mut Frame, app: &App, size: Rect) {
    let width = 60u16.min(size.width.saturating_sub(4));
    let height = 100u16.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let help_lines = vec![
        // ── Navigation ──────────────────────────────────────────────────────
        section_header("Navigation", &app.theme),
        key_line("j/Down", "Move down", &app.theme),
        key_line("k/Up", "Move up", &app.theme),
        key_line(
            "l/Right",
            "Enter dir / open file in new cmux tab",
            &app.theme,
        ),
        key_line("h/Left", "Go to parent", &app.theme),
        key_line("Enter", "Enter dir / open file in new cmux tab", &app.theme),
        key_line("g / G", "Go to top / bottom", &app.theme),
        key_line("~", "Go to home directory", &app.theme),
        key_line(".", "Toggle hidden files", &app.theme),
        key_line(
            "e",
            "Jump to path (Tab to complete, Enter to go)",
            &app.theme,
        ),
        key_line(
            "[ / ]",
            "Scroll preview pane up / down (5 lines)",
            &app.theme,
        ),
        key_line(
            "l/Right (on file)",
            "Enter preview focus mode (cursor into preview)",
            &app.theme,
        ),
        key_line(
            "Esc/h/Left (in preview)",
            "Exit preview focus, return to file tree",
            &app.theme,
        ),
        key_line(
            "j/k (in preview)",
            "Move cursor line down / up in preview focus",
            &app.theme,
        ),
        key_line(
            "J/K (in preview)",
            "Extend selection down / up in preview focus",
            &app.theme,
        ),
        key_line(
            "Enter (in preview)",
            "Open file in editor from preview focus",
            &app.theme,
        ),
        key_line(
            "Tab (in preview)",
            "Send selected line(s) to a cmux surface",
            &app.theme,
        ),
        key_line(
            "g/G (in preview)",
            "Jump to top / bottom in preview focus",
            &app.theme,
        ),
        key_line("Ctrl+O", "Go back in directory history", &app.theme),
        key_line("Ctrl+I", "Go forward in directory history", &app.theme),
        key_line(
            "`<c>",
            "Set mark 'c' — record current dir to slot c (a-z A-Z)",
            &app.theme,
        ),
        key_line(
            "'<c>",
            "Jump to mark 'c' — navigate to the marked directory",
            &app.theme,
        ),
        Line::from(""),
        // ── Search ──────────────────────────────────────────────────────────
        section_header("Search", &app.theme),
        key_line("/", "Fuzzy search", &app.theme),
        key_line("|", "Filter/narrow listing (case-insensitive)", &app.theme),
        key_line("Ctrl+F", "Content search (ripgrep)", &app.theme),
        key_line("Ctrl+P", "Recursive filename find", &app.theme),
        key_line("b", "Bookmark current directory", &app.theme),
        key_line("B", "Open bookmark picker", &app.theme),
        key_line(
            "z",
            "Open frecency jump list (auto-ranked recent dirs)",
            &app.theme,
        ),
        Line::from(""),
        // ── View ────────────────────────────────────────────────────────────
        section_header("View", &app.theme),
        key_line("#", "Toggle line numbers in preview pane", &app.theme),
        key_line(
            "i",
            "Toggle gitignore filter (hide ignored files)",
            &app.theme,
        ),
        key_line("d", "Toggle git diff preview", &app.theme),
        key_line("V", "Toggle git log preview (commit history)", &app.theme),
        key_line("D", "Toggle disk usage breakdown for directory", &app.theme),
        key_line(
            "I",
            "Watch mode (auto-refresh listing on filesystem changes)",
            &app.theme,
        ),
        key_line("f", "Compare two selected files (unified diff)", &app.theme),
        key_line("m", "Toggle file metadata view", &app.theme),
        key_line("H", "Toggle hash preview (SHA-256 checksum)", &app.theme),
        key_line("a", "Toggle hex dump view (binary inspection)", &app.theme),
        key_line("w", "Toggle preview pane (hide/show)", &app.theme),
        key_line("\\", "Toggle parent pane (hide/show)", &app.theme),
        key_line("T", "Toggle timestamps / sizes in listing", &app.theme),
        key_line("U", "Toggle preview word wrap", &app.theme),
        key_line("N", "Toggle directory item counts", &app.theme),
        key_line("P", "Edit file permissions (chmod)", &app.theme),
        key_line(
            "F",
            "Toggle change feed (live filesystem events)",
            &app.theme,
        ),
        key_line("R", "Refresh git status", &app.theme),
        key_line("S", "Cycle sort: Name/Size/Modified/Ext", &app.theme),
        key_line("s", "Toggle sort order ↑↓", &app.theme),
        Line::from(""),
        // ── Selection & Rename ──────────────────────────────────────────────
        section_header("Selection & Rename", &app.theme),
        key_line(
            "J / K",
            "Extend selection down / up (range select)",
            &app.theme,
        ),
        key_line("Space", "Toggle file selection", &app.theme),
        key_line("v", "Select all files", &app.theme),
        key_line("*", "Select files by glob pattern (e.g. *.rs)", &app.theme),
        key_line("n / F2", "Quick rename (inline bar pre-filled)", &app.theme),
        key_line("r", "Bulk rename selected files with regex", &app.theme),
        key_line("Esc", "Clear filter (if active) or selections", &app.theme),
        Line::from(""),
        // ── File Operations ─────────────────────────────────────────────────
        section_header("File Operations", &app.theme),
        key_line("o", "Open in $EDITOR (suspends TUI)", &app.theme),
        key_line("O", "Open with system default (background)", &app.theme),
        key_line("c / C", "Copy current / selected", &app.theme),
        key_line("x", "Cut current to clipboard", &app.theme),
        key_line("F9", "Inspect clipboard contents", &app.theme),
        key_line("p", "Paste clipboard into current dir", &app.theme),
        key_line(
            "Delete / X",
            "Trash current / selected (recoverable)",
            &app.theme,
        ),
        key_line("u", "Undo last trash operation", &app.theme),
        key_line("t", "New file (touch — create empty file)", &app.theme),
        key_line("W", "Duplicate selected entry in place", &app.theme),
        key_line("L", "Create symlink to selected entry", &app.theme),
        key_line("Z", "Extract archive to current directory", &app.theme),
        key_line(
            "E",
            "Create archive from selected files (tar.gz, zip, …)",
            &app.theme,
        ),
        key_line("M", "Make new directory", &app.theme),
        Line::from(""),
        // ── Yank & Misc ─────────────────────────────────────────────────────
        section_header("Yank & Misc", &app.theme),
        key_line("y / Y", "Yank relative / absolute path", &app.theme),
        key_line(
            "A",
            "Yank path (pick format: r=relative a=absolute f=filename p=parent)",
            &app.theme,
        ),
        key_line(":", "Open command palette", &app.theme),
        key_line("Q", "Quit", &app.theme),
        key_line("?", "Toggle this help", &app.theme),
        Line::from(""),
        // ── AI workflow ─────────────────────────────────────────────────────
        section_header("AI workflow", &app.theme),
        key_line(
            "Ctrl+B",
            "Export context bundle (selected files → clipboard for AI chat)",
            &app.theme,
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Right-click: open file in new cmux tab",
            Style::default().fg(app.theme.fg_dim),
        )),
        Line::from(Span::styled(
            "  Double-click: open file in new cmux pane to the right",
            Style::default().fg(app.theme.fg_dim),
        )),
        Line::from(Span::styled(
            "  Drag dividers to resize · scroll wheel on all panes",
            Style::default().fg(app.theme.fg_dim),
        )),
        Line::from(Span::styled(
            "  Any key to close",
            Style::default().fg(app.theme.fg_dim),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(app.theme.fg)
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

/// Render the background task manager panel in the preview pane area.
///
/// Shows all recent file operations (copy, move, extract) with their status.
/// Active operations show a spinner-style indicator; completed ones show a
/// summary or error. Press `c` to clear completed tasks, `j`/`k` to navigate,
/// `Esc`/`q` to close.
fn draw_task_manager_pane(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::task_manager::TaskStatus;

    let has_running = app.task_manager.has_running();
    let title = if has_running {
        " Task Manager [running] "
    } else {
        " Task Manager "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_warn));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.task_manager.tasks.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No background tasks",
            Style::default().fg(app.theme.fg_dim),
        )))
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(msg, inner);

        // Footer hint
        let hint = Paragraph::new(Line::from(Span::styled(
            "  Ctrl+T or q to close",
            Style::default().fg(app.theme.fg_dim),
        )));
        if inner.height >= 3 {
            let hint_area = Rect {
                y: inner.y + inner.height - 1,
                height: 1,
                ..inner
            };
            f.render_widget(hint, hint_area);
        }
        return;
    }

    let height = inner.height as usize;
    let total = app.task_manager.tasks.len();
    let selected = app.task_manager.selected;

    let scroll_offset = if selected < height {
        0
    } else {
        selected - height + 1
    };

    let mut rows: Vec<Line> = Vec::with_capacity(height);
    for (i, task) in app
        .task_manager
        .tasks
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(height.saturating_sub(1))
    {
        let is_selected = i == selected;

        let (status_sym, status_style) = match &task.status {
            TaskStatus::Running => ("⟳", Style::default().fg(app.theme.info)),
            TaskStatus::Done { .. } => ("✓", Style::default().fg(app.theme.ok)),
            TaskStatus::Failed { .. } => ("✗", Style::default().fg(app.theme.error)),
        };

        let kind_label = task.kind.label();

        let detail = match &task.status {
            TaskStatus::Running => task.label.clone(),
            TaskStatus::Done { summary } => summary.clone(),
            TaskStatus::Failed { error } => format!("Error: {}", error),
        };

        let elapsed = {
            let secs = task.started_at.elapsed().as_secs();
            if secs < 60 {
                format!("{}s", secs)
            } else {
                format!("{}m{}s", secs / 60, secs % 60)
            }
        };

        let row_style = if is_selected {
            Style::default().bg(app.theme.subtle_sel_bg)
        } else {
            Style::default()
        };

        let width = inner.width as usize;
        let prefix = format!("  {} {:8}  ", status_sym, kind_label);
        let suffix = format!("  {}", elapsed);
        let available = width.saturating_sub(prefix.len() + suffix.len());
        let detail_trunc = truncate_with_ellipsis(&detail, available);

        let line = Line::from(vec![
            Span::styled("  ", row_style),
            Span::styled(status_sym.to_string(), status_style.patch(row_style)),
            Span::styled(format!(" {:8}  ", kind_label), row_style),
            Span::styled(detail_trunc, row_style),
            Span::styled(
                format!("  {}", elapsed),
                Style::default().fg(app.theme.fg_dim).patch(row_style),
            ),
        ]);
        rows.push(line);
    }

    // Footer: scroll indicator + keybinding hints.
    let footer_text = if total > height.saturating_sub(1) {
        format!(
            "  {}/{} tasks  │  j/k navigate  c clear done  q close",
            selected + 1,
            total
        )
    } else {
        "  j/k navigate  c clear done  q close".to_string()
    };
    let footer = Line::from(Span::styled(
        footer_text,
        Style::default().fg(app.theme.fg_dim),
    ));

    let content_height = inner.height.saturating_sub(1) as usize;
    for (row_idx, line) in rows.into_iter().take(content_height).enumerate() {
        let row_area = Rect {
            y: inner.y + row_idx as u16,
            height: 1,
            ..inner
        };
        f.render_widget(Paragraph::new(line), row_area);
    }

    if inner.height >= 2 {
        let footer_area = Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        };
        f.render_widget(Paragraph::new(footer), footer_area);
    }
}

/// Render the session change summary in the center pane.
///
/// Shows files grouped as NEW / MODIFIED / DELETED with a checkpoint header.
fn draw_session_summary_pane(f: &mut Frame, app: &App, area: Rect) {
    use std::time::{Duration, SystemTime};

    // ── Build header ──────────────────────────────────────────────────────────
    let (checkpoint_label, file_count) = if let Some(ref snap) = app.session_snapshot {
        let elapsed = SystemTime::now()
            .duration_since(snap.taken_at)
            .unwrap_or(Duration::ZERO);
        let mins = elapsed.as_secs() / 60;
        let elapsed_str = if mins == 0 {
            "just now".to_string()
        } else if mins < 60 {
            format!("{} min ago", mins)
        } else {
            format!("{} hr ago", mins / 60)
        };
        let total = app.session_summary_total;
        (
            format!("checkpoint: {} — {} files changed", elapsed_str, total),
            total,
        )
    } else {
        ("no checkpoint yet".to_string(), 0)
    };

    let title = format!(" SESSION CHANGES  ({}) ", checkpoint_label);

    let block = Block::default()
        .title(title.as_str())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let empty_cache = Vec::new();
    let cache = app.session_summary_cache.as_deref().unwrap_or(&empty_cache);

    if file_count == 0 {
        let para = Paragraph::new(Line::from(Span::styled(
            "  No changes since checkpoint",
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(para, inner);
        return;
    }

    // ── Build list items ──────────────────────────────────────────────────────
    let new_count = count_by_kind(cache, &ChangeKind::New);
    let mod_count = count_by_kind(cache, &ChangeKind::Modified);
    let del_count = count_by_kind(cache, &ChangeKind::Deleted);
    let sel = app.session_summary_selected;

    let mut items: Vec<ListItem> = Vec::new();

    if new_count > 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  NEW ({})", new_count),
            Style::default()
                .fg(app.theme.event_new)
                .add_modifier(Modifier::BOLD),
        ))));
        for (idx, entry) in cache.iter().enumerate() {
            if entry.kind == ChangeKind::New {
                let name = entry.path.to_string_lossy();
                let size_label = format_tokens(entry.size);
                let style = if sel == idx {
                    Style::default()
                        .fg(app.theme.confirm_fg)
                        .bg(app.theme.event_new)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.event_new)
                };
                let text = format!(
                    "  ├── {:48}  {}",
                    truncate_with_ellipsis(&name, 48),
                    size_label
                );
                items.push(ListItem::new(Line::from(Span::styled(text, style))));
            }
        }
    }

    if mod_count > 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  MODIFIED ({})", mod_count),
            Style::default()
                .fg(app.theme.event_modified)
                .add_modifier(Modifier::BOLD),
        ))));
        for (idx, entry) in cache.iter().enumerate() {
            if entry.kind == ChangeKind::Modified {
                let name = entry.path.to_string_lossy();
                let delta = entry.size as i64 - entry.old_size as i64;
                let delta_label = if delta >= 0 {
                    format!("+{}", format_tokens(delta as u64))
                } else {
                    format!("-{}", format_tokens((-delta) as u64))
                };
                let style = if sel == idx {
                    Style::default()
                        .fg(app.theme.confirm_fg)
                        .bg(app.theme.event_modified)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.event_modified)
                };
                let text = format!(
                    "  ├── {:48}  {}",
                    truncate_with_ellipsis(&name, 48),
                    delta_label
                );
                items.push(ListItem::new(Line::from(Span::styled(text, style))));
            }
        }
    }

    if del_count > 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  DELETED ({})", del_count),
            Style::default()
                .fg(app.theme.event_deleted)
                .add_modifier(Modifier::BOLD),
        ))));
        for (idx, entry) in cache.iter().enumerate() {
            if entry.kind == ChangeKind::Deleted {
                let name = entry.path.to_string_lossy();
                let size_label = format!("was {}", format_tokens(entry.old_size));
                let style = if sel == idx {
                    Style::default()
                        .fg(app.theme.confirm_fg)
                        .bg(app.theme.event_deleted)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.event_deleted)
                };
                let text = format!(
                    "  ├── {:48}  {}",
                    truncate_with_ellipsis(&name, 48),
                    size_label
                );
                items.push(ListItem::new(Line::from(Span::styled(text, style))));
            }
        }
    }

    if app.session_summary_total > crate::app::session_snapshot::MAX_DIFF_ENTRIES {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(
                "  … and {} more",
                app.session_summary_total - crate::app::session_snapshot::MAX_DIFF_ENTRIES
            ),
            Style::default().fg(app.theme.fg_dim),
        ))));
    }

    let list = List::new(items);
    f.render_widget(list, inner);
}

/// Render the cmux surface picker as a centred overlay.
///
/// Shows all discoverable surfaces in the current workspace (excluding Trek
/// itself).  The user can type to filter by id/kind/title and press Enter to
/// send the selected preview lines.
fn draw_cmux_surface_picker(f: &mut Frame, app: &App, size: Rect) {
    const MAX_VISIBLE: usize = 10;
    let row_count = app
        .overlay
        .cmux_surface_filtered
        .len()
        .clamp(1, MAX_VISIBLE);
    // +5: query row + content preview row + hint row + top/bottom border
    let width = 62u16.min(size.width.saturating_sub(4));
    let height = (row_count as u16 + 5)
        .min(size.height.saturating_sub(4))
        .max(7);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focus))
        .title(Span::styled(
            " Send to surface ",
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // inner is split into: query row | list rows | content preview row | hint row
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // query / filter row
            Constraint::Min(1),    // list
            Constraint::Length(1), // content preview
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // ── Query row ─────────────────────────────────────────────────────────────
    let query_para = Paragraph::new(Line::from(vec![
        Span::styled(" Filter: ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            app.overlay.cmux_surface_query.as_str(),
            Style::default().fg(app.theme.input),
        ),
        Span::styled("█", Style::default().fg(app.theme.cursor)),
    ]));
    f.render_widget(query_para, inner_chunks[0]);

    // ── Surface list ──────────────────────────────────────────────────────────
    let list_area = inner_chunks[1];
    let visible_height = list_area.height as usize;

    let scroll = if app.overlay.cmux_surface_selected >= visible_height {
        app.overlay.cmux_surface_selected - visible_height + 1
    } else {
        0
    };

    if app.overlay.cmux_surface_filtered.is_empty() {
        let para = Paragraph::new(Line::from(Span::styled(
            "  (no surfaces match)",
            Style::default().fg(app.theme.fg_dim),
        )));
        f.render_widget(para, list_area);
    } else {
        let title_width = (inner.width as usize).saturating_sub(14);
        let items: Vec<ListItem> = app
            .overlay
            .cmux_surface_filtered
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(display_idx, &real_idx)| {
                let s = &app.overlay.cmux_surfaces[real_idx];
                let is_selected = display_idx + scroll == app.overlay.cmux_surface_selected;

                let icon = match s.kind.as_str() {
                    "terminal" => ">_",
                    "browser" => "[B]",
                    "markdown" => "=",
                    _ => "?",
                };

                let title_col = truncate_with_ellipsis(&s.title, title_width);

                if is_selected {
                    let style = Style::default()
                        .fg(app.theme.sel_fg)
                        .bg(app.theme.sel_bg)
                        .add_modifier(Modifier::BOLD);
                    ListItem::new(Line::from(vec![Span::styled(
                        format!(" {:<3}  {:<12}  {}", icon, s.id, title_col),
                        style,
                    )]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {:<3} ", icon),
                            Style::default().fg(app.theme.info),
                        ),
                        Span::styled(
                            format!(" {:<12}  ", s.id),
                            Style::default().fg(app.theme.fg_dim),
                        ),
                        Span::raw(title_col),
                    ]))
                }
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, list_area);
    }

    // ── Content preview row ───────────────────────────────────────────────────
    let (lo, hi) = match app.preview.preview_selection_anchor {
        Some(anchor) => (
            anchor.min(app.preview.preview_cursor),
            anchor.max(app.preview.preview_cursor),
        ),
        None => (app.preview.preview_cursor, app.preview.preview_cursor),
    };
    let lo = lo.min(app.preview.preview_lines.len().saturating_sub(1));
    let hi = hi.min(app.preview.preview_lines.len().saturating_sub(1));
    let line_count = hi - lo + 1;
    let first_line = app
        .preview
        .preview_lines
        .get(lo)
        .map(|s| s.as_str())
        .unwrap_or("");
    let max_preview = (inner.width as usize).saturating_sub(16);
    let preview_text = truncate_with_ellipsis(first_line.trim(), max_preview);
    let content_para = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(
                " {} line{}  ",
                line_count,
                if line_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(app.theme.fg_dim),
        ),
        Span::styled(
            format!("\"{}\"", preview_text),
            Style::default()
                .fg(app.theme.fg_dim)
                .add_modifier(Modifier::ITALIC),
        ),
    ]));
    f.render_widget(content_para, inner_chunks[2]);

    // ── Hint row ──────────────────────────────────────────────────────────────
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            "Enter",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" send  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel  ", Style::default().fg(app.theme.fg_dim)),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " navigate  type to filter",
            Style::default().fg(app.theme.fg_dim),
        ),
    ]));
    f.render_widget(hint, inner_chunks[3]);
}
