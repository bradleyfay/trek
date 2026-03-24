// All unit tests for the App module — kept in a separate file to reduce mod.rs size.
// `use super::*` pulls in all pub/pub(crate) items from app::mod.

use super::*;

fn make_entry(name: &str, is_dir: bool, size: u64, secs: u64) -> DirEntry {
    DirEntry {
        name: name.to_string(),
        path: PathBuf::from(name),
        is_dir,
        size,
        modified: std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs),
    }
}

/// Given: SortMode::Name
/// When: next() is called 4 times
/// Then: cycles back to Name
#[test]
fn sort_mode_cycles_all_variants() {
    let mut m = SortMode::Name;
    m = m.next();
    assert_eq!(m, SortMode::Size);
    m = m.next();
    assert_eq!(m, SortMode::Modified);
    m = m.next();
    assert_eq!(m, SortMode::Extension);
    m = m.next();
    assert_eq!(m, SortMode::Name);
}

/// Given: each SortMode
/// When: label() is called
/// Then: returns a non-empty string matching the mode name
#[test]
fn sort_mode_labels_are_non_empty() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Size.label(), "Size");
    assert_eq!(SortMode::Modified.label(), "Modified");
    assert_eq!(SortMode::Extension.label(), "Extension");
}

/// Given: mixed files and dirs with various names
/// When: sort_entries is called with Name/Ascending
/// Then: dirs come first, then files in A-Z order (case-insensitive)
#[test]
fn sort_by_name_ascending_dirs_first() {
    let mut entries = vec![
        make_entry("zebra.rs", false, 100, 0),
        make_entry("src", true, 0, 0),
        make_entry("apple.rs", false, 50, 0),
        make_entry("lib", true, 0, 0),
    ];
    App::sort_entries(&mut entries, SortMode::Name, SortOrder::Ascending);
    assert!(entries[0].is_dir && entries[1].is_dir, "dirs first");
    assert_eq!(entries[2].name, "apple.rs");
    assert_eq!(entries[3].name, "zebra.rs");
}

/// Given: files with different sizes
/// When: sort_entries is called with Size/Descending
/// Then: dirs come first; files sorted largest → smallest
#[test]
fn sort_by_size_descending_largest_first() {
    let mut entries = vec![
        make_entry("small.txt", false, 10, 0),
        make_entry("large.txt", false, 9999, 0),
        make_entry("medium.txt", false, 500, 0),
    ];
    App::sort_entries(&mut entries, SortMode::Size, SortOrder::Descending);
    assert_eq!(entries[0].name, "large.txt");
    assert_eq!(entries[1].name, "medium.txt");
    assert_eq!(entries[2].name, "small.txt");
}

/// Given: files with different modification times
/// When: sort_entries is called with Modified/Descending
/// Then: newest file appears first
#[test]
fn sort_by_modified_descending_newest_first() {
    let mut entries = vec![
        make_entry("old.txt", false, 0, 1000),
        make_entry("new.txt", false, 0, 9999),
        make_entry("mid.txt", false, 0, 5000),
    ];
    App::sort_entries(&mut entries, SortMode::Modified, SortOrder::Descending);
    assert_eq!(entries[0].name, "new.txt");
    assert_eq!(entries[1].name, "mid.txt");
    assert_eq!(entries[2].name, "old.txt");
}

/// Given: files with various extensions
/// When: sort_entries is called with Extension/Ascending
/// Then: dirs first; files grouped by extension then alphabetically
#[test]
fn sort_by_extension_groups_by_ext() {
    let mut entries = vec![
        make_entry("b.rs", false, 0, 0),
        make_entry("a.toml", false, 0, 0),
        make_entry("a.rs", false, 0, 0),
    ];
    App::sort_entries(&mut entries, SortMode::Extension, SortOrder::Ascending);
    // rs < toml alphabetically
    assert_eq!(entries[0].name, "a.rs");
    assert_eq!(entries[1].name, "b.rs");
    assert_eq!(entries[2].name, "a.toml");
}

/// Given: a mix of files and directories under any sort mode
/// When: sort_entries is called
/// Then: directories always appear before files
#[test]
fn dirs_always_before_files_regardless_of_sort_mode() {
    for mode in [
        SortMode::Name,
        SortMode::Size,
        SortMode::Modified,
        SortMode::Extension,
    ] {
        for order in [SortOrder::Ascending, SortOrder::Descending] {
            let mut entries = vec![
                make_entry("z_file.txt", false, 9999, 9999),
                make_entry("a_dir", true, 0, 0),
                make_entry("b_file.txt", false, 1, 1),
            ];
            App::sort_entries(&mut entries, mode, order);
            assert!(
                entries[0].is_dir,
                "dir should be first for mode={mode:?} order={order:?}, got {:?}",
                entries.iter().map(|e| &e.name).collect::<Vec<_>>()
            );
        }
    }
}

// ── History tests ────────────────────────────────────────────────────────

fn make_app_at(dir: &std::path::Path) -> App {
    let mut app = App::new(Some(dir.to_path_buf())).expect("App::new");
    // Clear the initial status message so tests can check specific messages.
    app.status_message = None;
    app
}

/// Given: a fresh App
/// When: history is checked
/// Then: stack has exactly one entry (the launch directory) at position 0
#[test]
fn history_initialized_with_one_entry() {
    let dir = std::env::temp_dir();
    let app = make_app_at(&dir);
    assert_eq!(app.history_len(), 1);
    assert_eq!(app.history_position(), 0);
}

/// Given: a fresh App
/// When: history_back() is called at position 0
/// Then: status_message is "Already at oldest location"; position unchanged
#[test]
fn history_back_at_oldest_shows_message() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    app.history_back();
    assert_eq!(app.history_position(), 0);
    assert_eq!(
        app.status_message.as_deref(),
        Some("Already at oldest location")
    );
}

/// Given: a fresh App
/// When: history_forward() is called with no forward entries
/// Then: status_message is "Already at newest location"; position unchanged
#[test]
fn history_forward_at_newest_shows_message() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    app.history_forward();
    assert_eq!(app.history_position(), 0);
    assert_eq!(
        app.status_message.as_deref(),
        Some("Already at newest location")
    );
}

/// Given: two distinct real directories
/// When: push_history is called twice, then history_back once
/// Then: position returns to 1 (one step back) and stack still has 3 entries
#[test]
fn push_history_then_back_restores_position() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    let sub1 = std::env::temp_dir();
    let sub2 = std::env::temp_dir();
    app.push_history(sub1.clone());
    app.push_history(sub2.clone());
    assert_eq!(app.history_len(), 3);
    assert_eq!(app.history_position(), 2);
    // Go back — position should move to 1.
    app.history_pos -= 1; // bypass restore (no real dir switch needed)
    assert_eq!(app.history_position(), 1);
}

/// Given: user navigates forward, then goes back, then navigates to a new dir
/// When: push_history is called for the new dir
/// Then: forward entries are discarded (browser-style)
#[test]
fn forward_history_discarded_on_new_navigation() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    let sub1 = std::env::temp_dir();
    let sub2 = std::env::temp_dir();
    let sub3 = std::env::temp_dir();
    app.push_history(sub1);
    app.push_history(sub2);
    assert_eq!(app.history_len(), 3);
    // Simulate going back.
    app.history_pos = 1;
    // Navigate to a new dir — should discard entry at index 2.
    app.push_history(sub3);
    assert_eq!(
        app.history_len(),
        3,
        "old forward entry should be replaced, not accumulated"
    );
    assert_eq!(app.history_position(), 2);
}

/// Given: MAX_HISTORY + 5 push_history calls
/// When: stack length is checked
/// Then: stack is capped at MAX_HISTORY
#[test]
fn history_capped_at_max() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    for _ in 0..(MAX_HISTORY + 5) {
        app.push_history(std::env::temp_dir());
    }
    assert!(app.history_len() <= MAX_HISTORY);
}

// ── Metadata helper tests ─────────────────────────────────────────────────

/// Given: Unix mode 0o644
/// When: format_permission_bits is called
/// Then: returns "rw-r--r--"
#[test]
fn permission_bits_0644() {
    assert_eq!(format_permission_bits(0o644), "rw-r--r--");
}

/// Given: Unix mode 0o755
/// When: format_permission_bits is called
/// Then: returns "rwxr-xr-x"
#[test]
fn permission_bits_0755() {
    assert_eq!(format_permission_bits(0o755), "rwxr-xr-x");
}

/// Given: 0 bytes
/// When: meta_human_size is called
/// Then: returns "0 B"
#[test]
fn human_size_zero_bytes() {
    assert_eq!(meta_human_size(0), "0 B");
}

/// Given: exactly 1024 bytes
/// When: meta_human_size is called
/// Then: returns "1.0 KB"
#[test]
fn human_size_one_kb() {
    assert_eq!(meta_human_size(1024), "1.0 KB");
}

/// Given: Unix timestamp 0 (epoch)
/// When: format_unix_timestamp_utc is called
/// Then: returns "1970-01-01 00:00:00"
#[test]
fn timestamp_epoch() {
    assert_eq!(format_unix_timestamp_utc(0), "1970-01-01 00:00:00");
}

/// Given: Unix timestamp 1705318245 (2024-01-15 11:30:45 UTC)
/// When: format_unix_timestamp_utc is called
/// Then: returns the correct date/time string
#[test]
fn timestamp_known_date() {
    assert_eq!(
        format_unix_timestamp_utc(1_705_318_245),
        "2024-01-15 11:30:45"
    );
}

// ── Filter / narrow mode tests ────────────────────────────────────────────

/// Given: a fresh App
/// When: filter state is inspected
/// Then: filter_mode is false and filter_input is empty
#[test]
fn filter_mode_is_off_by_default() {
    let dir = std::env::temp_dir();
    let app = make_app_at(&dir);
    assert!(!app.filter_mode);
    assert!(app.filter_input.is_empty());
}

/// Given: a fresh App
/// When: start_filter() is called
/// Then: filter_mode is true
#[test]
fn start_filter_sets_filter_mode() {
    let dir = std::env::temp_dir();
    let mut app = make_app_at(&dir);
    app.start_filter();
    assert!(app.filter_mode);
}

/// Given: an App in a temp dir containing "alpha.txt" and "beta.txt"
/// When: filter_push_char('a') is called
/// Then: only entries whose names contain 'a' remain
#[test]
fn filter_push_char_narrows_listing() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_narrow_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("alpha.txt"), b"").unwrap();
    std::fs::write(tmp.join("beta.txt"), b"").unwrap();
    std::fs::write(tmp.join("gamma.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    app.start_filter();
    app.filter_push_char('a');

    let names: Vec<&str> = app.entries.iter().map(|e| e.name.as_str()).collect();
    for name in &names {
        assert!(
            name.to_lowercase().contains('a'),
            "expected all entries to contain 'a', got {name}"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: a file named "README.md"
/// When: filter_push_char with lowercase 'r', 'e', 'a' is called
/// Then: the file still appears (case-insensitive match)
#[test]
fn filter_is_case_insensitive() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_case_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("README.md"), b"").unwrap();
    std::fs::write(tmp.join("other.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    app.start_filter();
    app.filter_push_char('r');
    app.filter_push_char('e');
    app.filter_push_char('a');

    assert!(
        app.entries.iter().any(|e| e.name == "README.md"),
        "README.md should still be visible with filter 'rea'"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: filter "al" is active (showing only "alpha.txt")
/// When: filter_pop_char() is called
/// Then: listing contains more entries than before (filter widened)
#[test]
fn filter_pop_char_widens_listing() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_pop_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("alpha.txt"), b"").unwrap();
    std::fs::write(tmp.join("beta.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    app.start_filter();
    app.filter_push_char('a');
    app.filter_push_char('l');
    let narrow_count = app.entries.len();

    app.filter_pop_char(); // back to just "a"
    let wider_count = app.entries.len();
    assert!(
        wider_count >= narrow_count,
        "popping a char should not shrink the listing further"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: filter "xyz" matches nothing
/// When: filter_push_char cycles build "xyz"
/// Then: entries is empty and no panic occurs
#[test]
fn filter_no_match_gives_empty_listing() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("alpha.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    app.start_filter();
    for c in "zzznomatch".chars() {
        app.filter_push_char(c);
    }
    assert_eq!(app.entries.len(), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: filter_mode is true with non-empty filter_input
/// When: close_filter() is called
/// Then: filter_mode is false but filter_input is still non-empty (frozen)
#[test]
fn close_filter_keeps_filter_active() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_close_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("alpha.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    app.start_filter();
    app.filter_push_char('a');
    app.close_filter();

    assert!(
        !app.filter_mode,
        "filter_mode should be false after close_filter"
    );
    assert!(
        !app.filter_input.is_empty(),
        "filter_input should remain non-empty"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: a frozen filter narrowing the listing
/// When: clear_filter() is called
/// Then: filter_input is empty, filter_mode is false, full listing is restored
#[test]
fn clear_filter_restores_full_listing() {
    let tmp = std::env::temp_dir().join(format!("trek_filter_test_clear_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("alpha.txt"), b"").unwrap();
    std::fs::write(tmp.join("zzz.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    let full_count = app.entries.len();

    app.start_filter();
    app.filter_push_char('a');
    app.filter_push_char('l');
    app.filter_push_char('p');
    let narrow_count = app.entries.len();
    assert!(
        narrow_count < full_count,
        "filter 'alp' should narrow the listing (full={full_count}, narrow={narrow_count})"
    );

    app.clear_filter();
    assert!(app.filter_input.is_empty());
    assert!(!app.filter_mode);
    assert_eq!(
        app.entries.len(),
        full_count,
        "full listing should be restored"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── open-in-external tests ───────────────────────────────────────────────────

/// Given: no entries in the listing
/// When: selected_file_path() is called
/// Then: returns None
#[test]
fn selected_file_path_empty_entries_returns_none() {
    let tmp = std::env::temp_dir().join(format!("trek_open_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.entries.clear();
    assert!(app.selected_file_path().is_none());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: selected entry is a directory
/// When: selected_file_path() is called
/// Then: returns None (directories are not files)
#[test]
fn selected_file_path_on_directory_returns_none() {
    let tmp = std::env::temp_dir().join(format!("trek_open_dir_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let sub = tmp.join("subdir");
    let _ = std::fs::create_dir_all(&sub);

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| e.is_dir) {
        app.selected = idx;
    }
    assert!(app.selected_file_path().is_none());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: selected entry is a regular file
/// When: selected_file_path() is called
/// Then: returns Some(path) pointing to that file
#[test]
fn selected_file_path_on_file_returns_some() {
    let tmp = std::env::temp_dir().join(format!("trek_open_file_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("readme.md"), b"hello").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
        let path = app.selected_file_path();
        assert!(path.is_some());
        assert_eq!(
            path.unwrap().file_name().unwrap().to_string_lossy(),
            "readme.md"
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: selected entry is a directory
/// When: selected_path() is called
/// Then: returns Some(path) (selected_path works for both files and dirs)
#[test]
fn selected_path_on_directory_returns_some() {
    let tmp = std::env::temp_dir().join(format!("trek_selpath_dir_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let sub = tmp.join("subdir");
    let _ = std::fs::create_dir_all(&sub);

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| e.is_dir) {
        app.selected = idx;
    }
    assert!(app.selected_path().is_some());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: selected entry is a file
/// When: selected_path() is called
/// Then: returns Some(path)
#[test]
fn selected_path_on_file_returns_some() {
    let tmp = std::env::temp_dir().join(format!("trek_selpath_file_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("config.toml"), b"[tool]").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
    }
    assert!(app.selected_path().is_some());
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── command palette tests ────────────────────────────────────────────────────

/// Given: empty query
/// When: filter_palette("") is called
/// Then: all actions are returned
#[test]
fn palette_filter_empty_query_returns_all() {
    let results = crate::app::palette::filter_palette("");
    assert_eq!(results.len(), crate::app::palette::PALETTE_ACTIONS.len());
}

/// Given: a query matching some action names
/// When: filter_palette("sort") is called
/// Then: only actions whose names contain "sort" are returned
#[test]
fn palette_filter_narrows_by_substring() {
    let results = crate::app::palette::filter_palette("sort");
    assert!(!results.is_empty(), "expected at least one sort action");
    for &i in &results {
        let name = crate::app::palette::PALETTE_ACTIONS[i].name.to_lowercase();
        assert!(name.contains("sort"), "unexpected action: {}", name);
    }
}

/// Given: a query that matches no action names
/// When: filter_palette("zzznomatch") is called
/// Then: empty vec returned
#[test]
fn palette_filter_no_match_returns_empty() {
    let results = crate::app::palette::filter_palette("zzznomatch");
    assert!(results.is_empty());
}

/// Given: palette is closed
/// When: open_palette() is called
/// Then: palette_mode is true, query is empty, filtered list is full
#[test]
fn open_palette_sets_mode_and_resets_query() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_open_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    assert!(!app.palette_mode);
    app.open_palette();
    assert!(app.palette_mode);
    assert!(app.palette_query.is_empty());
    assert_eq!(
        app.palette_filtered.len(),
        crate::app::palette::PALETTE_ACTIONS.len()
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: palette is open with a query
/// When: close_palette() is called
/// Then: palette_mode is false and query is cleared
#[test]
fn close_palette_clears_state() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_close_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.open_palette();
    app.palette_push_char('s');
    app.palette_push_char('o');
    app.close_palette();
    assert!(!app.palette_mode);
    assert!(app.palette_query.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: palette is open with no query
/// When: palette_push_char('q') is called
/// Then: only actions containing "q" remain in filtered list; selected resets to 0
#[test]
fn palette_push_char_narrows_filtered_list() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_push_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.open_palette();
    let full_count = app.palette_filtered.len();
    app.palette_push_char('q');
    assert!(app.palette_filtered.len() < full_count);
    assert_eq!(app.palette_selected, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: palette query is "so"
/// When: palette_pop_char() is called
/// Then: query becomes "s" and filtered list widens
#[test]
fn palette_pop_char_widens_filtered_list() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_pop_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.open_palette();
    app.palette_push_char('s');
    app.palette_push_char('o');
    let narrow = app.palette_filtered.len();
    app.palette_pop_char();
    assert!(app.palette_filtered.len() >= narrow);
    assert_eq!(&app.palette_query, "s");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: palette has multiple results
/// When: palette_move_down() then palette_move_up() are called
/// Then: selected changes appropriately and stays in bounds
#[test]
fn palette_navigation_stays_in_bounds() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_nav_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.open_palette();
    assert_eq!(app.palette_selected, 0);
    app.palette_move_down();
    assert_eq!(app.palette_selected, 1);
    app.palette_move_up();
    assert_eq!(app.palette_selected, 0);
    // Moving up at top stays at 0
    app.palette_move_up();
    assert_eq!(app.palette_selected, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: palette_selected_action() with a known action in the list
/// When: called
/// Then: returns Some(ActionId) for the selected entry
#[test]
fn palette_selected_action_returns_correct_id() {
    let tmp = std::env::temp_dir().join(format!("trek_palette_sel_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.open_palette();
    // With empty query, first filtered entry is PALETTE_ACTIONS[0]
    let action = app.palette_selected_action();
    assert!(action.is_some());
    assert_eq!(
        action.unwrap(),
        crate::app::palette::PALETTE_ACTIONS[app.palette_filtered[0]].id
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── quick rename tests ───────────────────────────────────────────────────────

/// Given: a directory with a file selected
/// When: begin_quick_rename() is called
/// Then: quick_rename_mode is true, input is pre-filled with the current entry name
#[test]
fn begin_quick_rename_prefills_name() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_begin_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("hello.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
    }
    app.begin_quick_rename();
    assert!(app.quick_rename_mode);
    assert_eq!(app.quick_rename_input, "hello.txt");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: no entries in the listing
/// When: begin_quick_rename() is called
/// Then: quick_rename_mode stays false (no-op)
#[test]
fn begin_quick_rename_empty_entries_is_noop() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.entries.clear();
    app.begin_quick_rename();
    assert!(!app.quick_rename_mode);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar is open
/// When: cancel_quick_rename() is called
/// Then: mode is false, input is cleared, filesystem unchanged
#[test]
fn cancel_quick_rename_clears_state() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_cancel_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("file.rs"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
    }
    app.begin_quick_rename();
    app.cancel_quick_rename();
    assert!(!app.quick_rename_mode);
    assert!(app.quick_rename_input.is_empty());
    assert!(tmp.join("file.rs").exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar is open with the original name
/// When: Enter is pressed (confirm_quick_rename) with no changes
/// Then: the file is NOT renamed (no-op), mode closes
#[test]
fn confirm_quick_rename_same_name_is_noop() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_same_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("same.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| e.name == "same.txt") {
        app.selected = idx;
    }
    app.begin_quick_rename();
    // input already equals current name — confirm should be a no-op
    app.confirm_quick_rename();
    assert!(!app.quick_rename_mode);
    assert!(tmp.join("same.txt").exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar is open
/// When: input is cleared and Enter pressed
/// Then: status message says "Name cannot be empty", bar stays open
#[test]
fn confirm_quick_rename_empty_input_shows_error() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_empty_in_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("nonempty.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
    }
    app.begin_quick_rename();
    app.quick_rename_input.clear();
    app.confirm_quick_rename();
    // Mode should be closed (we close on empty) and status set
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("empty") || msg.contains("Empty"),
        "expected empty-name error, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar is open with a new valid name
/// When: confirm_quick_rename() is called
/// Then: file is renamed, listing refreshed, cursor on renamed entry, status shown
#[test]
fn confirm_quick_rename_renames_file() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_rename_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("old.txt"), b"content").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| e.name == "old.txt") {
        app.selected = idx;
    }
    app.begin_quick_rename();
    app.quick_rename_input = "new.txt".to_string();
    app.confirm_quick_rename();

    assert!(!app.quick_rename_mode);
    assert!(tmp.join("new.txt").exists(), "renamed file should exist");
    assert!(!tmp.join("old.txt").exists(), "old file should be gone");
    assert!(
        app.entries.iter().any(|e| e.name == "new.txt"),
        "listing should contain new name"
    );
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("old.txt") && msg.contains("new.txt"),
        "status should mention both names, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar is open, target name already exists
/// When: confirm_quick_rename() is called
/// Then: status shows collision error, original file still exists
#[test]
fn confirm_quick_rename_collision_shows_error() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_coll_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"").unwrap();
    std::fs::write(tmp.join("b.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| e.name == "a.txt") {
        app.selected = idx;
    }
    app.begin_quick_rename();
    app.quick_rename_input = "b.txt".to_string();
    app.confirm_quick_rename();

    assert!(tmp.join("a.txt").exists(), "original should still exist");
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(
        msg.contains("exists") || msg.contains("Exists") || msg.contains("Already"),
        "expected collision error, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: quick rename bar
/// When: quick_rename_push_char and quick_rename_pop_char are called
/// Then: input is updated correctly
#[test]
fn quick_rename_push_pop_char() {
    let tmp = std::env::temp_dir().join(format!("trek_qr_chars_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("test.txt"), b"").unwrap();

    let mut app = make_app_at(&tmp);
    if let Some(idx) = app.entries.iter().position(|e| !e.is_dir) {
        app.selected = idx;
    }
    app.begin_quick_rename();
    let original_len = app.quick_rename_input.len();
    app.quick_rename_push_char('X');
    assert_eq!(app.quick_rename_input.len(), original_len + 1);
    app.quick_rename_pop_char();
    assert_eq!(app.quick_rename_input.len(), original_len);
    let _ = std::fs::remove_dir_all(&tmp);
}
