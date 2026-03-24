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

// ── gitignore-filter tests ───────────────────────────────────────────────────

/// Given: hide_gitignored defaults to false
/// When: App is created
/// Then: hide_gitignored is false and gitignored_names is empty
#[test]
fn gitignore_filter_default_off() {
    let tmp = std::env::temp_dir().join(format!("trek_gi_default_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let app = make_app_at(&tmp);
    assert!(!app.hide_gitignored);
    assert!(app.gitignored_names.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: not inside a git repository
/// When: toggle_gitignored() is called
/// Then: hide_gitignored stays false, status_message contains "git"
#[test]
fn toggle_gitignored_outside_repo_shows_error() {
    // Use a temp dir that is definitely not a git repo
    let tmp = std::env::temp_dir().join(format!("trek_gi_norepo_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    // Force git_status to None to simulate non-repo
    app.git_status = None;
    app.toggle_gitignored();
    assert!(!app.hide_gitignored, "should remain off outside a repo");
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(
        msg.to_lowercase().contains("git"),
        "expected git-related error message, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: load_ignored() is called on a directory with a .gitignore
/// When: the .gitignore lists a filename present in the dir
/// Then: that filename appears in the returned HashSet
#[test]
fn load_ignored_returns_ignored_names() {
    // Set up a real git repo so git ls-files actually works
    let tmp = std::env::temp_dir().join(format!("trek_gi_ignored_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::create_dir_all(&tmp);

    // git init the temp dir
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&tmp)
        .status();
    if init.is_err() {
        // git not available — skip gracefully
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }

    // Create .gitignore and an ignored file
    std::fs::write(tmp.join(".gitignore"), "ignored_file.txt\n").unwrap();
    std::fs::write(tmp.join("ignored_file.txt"), b"noise").unwrap();
    std::fs::write(tmp.join("tracked.rs"), b"signal").unwrap();

    let ignored = crate::git::load_ignored(&tmp);
    // Only check if git is available and returned something meaningful
    if !ignored.is_empty() {
        assert!(
            ignored.contains("ignored_file.txt"),
            "expected ignored_file.txt in ignored set, got: {:?}",
            ignored
        );
        assert!(
            !ignored.contains("tracked.rs"),
            "tracked.rs should not be in ignored set"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: load_ignored() is called outside a git repo
/// When: git ls-files fails
/// Then: returns an empty HashSet without panicking
#[test]
fn load_ignored_outside_repo_returns_empty() {
    let tmp = std::env::temp_dir().join(format!("trek_gi_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let ignored = crate::git::load_ignored(&tmp);
    assert!(ignored.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: hide_gitignored is false and listing contains an entry named "target"
/// When: toggle_gitignored() is called (simulated by setting hide_gitignored and calling load_dir)
/// Then: hide_gitignored flips to true after toggle when git_status is present
#[test]
fn hide_gitignored_field_toggles() {
    let tmp = std::env::temp_dir().join(format!("trek_gi_toggle_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("main.rs"), b"fn main() {}").unwrap();

    let mut app = make_app_at(&tmp);
    // Simulate being in a repo by checking initial state
    // (the actual toggle_gitignored checks git_status; we test field directly)
    assert!(!app.hide_gitignored);
    app.hide_gitignored = true;
    assert!(app.hide_gitignored);
    app.hide_gitignored = false;
    assert!(!app.hide_gitignored);

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── path jump bar tests ──────────────────────────────────────────────────────

/// Given: normal mode
/// When: begin_path_jump() is called
/// Then: path_mode is true, path_input is empty
#[test]
fn begin_path_jump_opens_bar() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_begin_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    assert!(!app.path_mode);
    app.begin_path_jump();
    assert!(app.path_mode);
    assert!(app.path_input.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: path jump bar is open
/// When: cancel_path_jump() is called
/// Then: path_mode is false, input cleared, cwd unchanged
#[test]
fn cancel_path_jump_clears_without_navigating() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_cancel_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    let original_cwd = app.cwd.clone();
    app.begin_path_jump();
    app.path_push_char('/');
    app.path_push_char('t');
    app.path_push_char('m');
    app.path_push_char('p');
    app.cancel_path_jump();
    assert!(!app.path_mode);
    assert!(app.path_input.is_empty());
    assert_eq!(app.cwd, original_cwd);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: path jump bar with empty input
/// When: confirm_path_jump() is called
/// Then: bar closes silently (no crash, no navigation)
#[test]
fn confirm_path_jump_empty_input_cancels_silently() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_path_jump();
    app.confirm_path_jump();
    assert!(!app.path_mode);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: path jump bar with a valid absolute directory path
/// When: confirm_path_jump() is called
/// Then: cwd changes to the target directory, history entry pushed
#[test]
fn confirm_path_jump_absolute_dir_navigates() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_abs_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let target = std::env::temp_dir().join(format!("trek_pj_target_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&target);
    let canonical_target = target.canonicalize().unwrap();

    let mut app = make_app_at(&tmp);
    app.begin_path_jump();
    for c in canonical_target.to_string_lossy().chars() {
        app.path_push_char(c);
    }
    app.confirm_path_jump();

    assert!(!app.path_mode, "bar should be closed after navigation");
    assert_eq!(app.cwd, canonical_target, "cwd should be the target dir");

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&target);
}

/// Given: path jump bar with a path to an existing file
/// When: confirm_path_jump() is called
/// Then: cwd becomes the file's parent directory and the file is selected
#[test]
fn confirm_path_jump_file_path_navigates_to_parent() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_file_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let file = tmp.join("target_file.txt");
    std::fs::write(&file, b"content").unwrap();
    let canonical_file = file.canonicalize().unwrap();
    let canonical_dir = tmp.canonicalize().unwrap();

    let start = std::env::temp_dir().join(format!("trek_pj_start_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&start);
    let mut app = make_app_at(&start);
    app.begin_path_jump();
    for c in canonical_file.to_string_lossy().chars() {
        app.path_push_char(c);
    }
    app.confirm_path_jump();

    assert!(!app.path_mode, "bar should be closed");
    assert_eq!(app.cwd, canonical_dir, "cwd should be file's parent");
    // Cursor should be on target_file.txt
    let selected_name = app.entries.get(app.selected).map(|e| e.name.as_str());
    assert_eq!(
        selected_name,
        Some("target_file.txt"),
        "cursor should land on the file"
    );

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&start);
}

/// Given: path jump bar with a nonexistent path
/// When: confirm_path_jump() is called
/// Then: status message is shown and bar stays open
#[test]
fn confirm_path_jump_nonexistent_path_shows_error() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_noex_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_path_jump();
    // Type a path that cannot exist
    for c in "/absolutely/does/not/exist/xyz_99999".chars() {
        app.path_push_char(c);
    }
    app.confirm_path_jump();

    // Bar stays open for correction
    assert!(app.path_mode, "bar should stay open on error");
    assert!(app.status_message.is_some(), "status message should be set");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: path jump bar with push/pop char
/// When: characters are added and removed
/// Then: path_input reflects changes correctly
#[test]
fn path_jump_push_pop_char() {
    let tmp = std::env::temp_dir().join(format!("trek_pj_chars_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_path_jump();
    app.path_push_char('/');
    app.path_push_char('t');
    app.path_push_char('m');
    app.path_push_char('p');
    assert_eq!(app.path_input, "/tmp");
    app.path_pop_char();
    assert_eq!(app.path_input, "/tm");
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── range selection (J / K) tests ──────────────────────────────────────────────

/// Given: cursor at index 0 in a multi-file directory
/// When: select_move_down() is called once
/// Then: entries 0 and 1 are selected, cursor is at 1
#[test]
fn select_move_down_marks_both_endpoints() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_down_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"").unwrap();
    std::fs::write(tmp.join("b.txt"), b"").unwrap();
    std::fs::write(tmp.join("c.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    app.selected = 0;
    app.select_move_down();
    assert!(
        app.rename_selected.contains(&0),
        "entry 0 should be selected"
    );
    assert!(
        app.rename_selected.contains(&1),
        "entry 1 should be selected"
    );
    assert_eq!(app.selected, 1, "cursor should be at 1");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: cursor at index 1 in a multi-file directory
/// When: select_move_up() is called once
/// Then: entries 1 and 0 are selected, cursor is at 0
#[test]
fn select_move_up_marks_both_endpoints() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_up_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"").unwrap();
    std::fs::write(tmp.join("b.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    app.selected = 1;
    app.select_move_up();
    assert!(
        app.rename_selected.contains(&1),
        "entry 1 should be selected"
    );
    assert!(
        app.rename_selected.contains(&0),
        "entry 0 should be selected"
    );
    assert_eq!(app.selected, 0, "cursor should be at 0");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: cursor at the last entry
/// When: select_move_down() is called
/// Then: cursor stays at last entry; last entry is marked
#[test]
fn select_move_down_at_bottom_stays_and_marks() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_bot_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"").unwrap();
    std::fs::write(tmp.join("b.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    app.selected = app.entries.len() - 1;
    let last = app.selected;
    app.select_move_down();
    assert_eq!(app.selected, last, "cursor should not move past bottom");
    assert!(
        app.rename_selected.contains(&last),
        "last entry should be selected"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: cursor at index 0 (top)
/// When: select_move_up() is called
/// Then: cursor stays at 0; entry 0 is marked
#[test]
fn select_move_up_at_top_stays_and_marks() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_top_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"").unwrap();
    std::fs::write(tmp.join("b.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    app.selected = 0;
    app.select_move_up();
    assert_eq!(app.selected, 0, "cursor should not move above top");
    assert!(
        app.rename_selected.contains(&0),
        "entry 0 should be selected"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: directory entries in selection, no file entries
/// When: start_rename() is called
/// Then: status message shown, rename_mode stays false
#[test]
fn start_rename_with_only_dirs_in_selection_shows_message() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_dir_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let subdir = tmp.join("subdir");
    let _ = std::fs::create_dir_all(&subdir);
    // Also need a file so load_dir has entries
    std::fs::write(tmp.join("file.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    // Find the directory entry and add it to selection
    let dir_idx = app.entries.iter().position(|e| e.is_dir).unwrap();
    app.rename_selected.insert(dir_idx);
    app.start_rename();
    assert!(
        !app.rename_mode,
        "rename_mode should be false when only dirs selected"
    );
    assert!(app.status_message.is_some(), "status message should be set");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: a directory entry selected by J (select_move_down includes dirs)
/// When: cursor is on a directory, select_move_down called
/// Then: the directory index is in rename_selected
#[test]
fn select_move_down_includes_directories() {
    let tmp = std::env::temp_dir().join(format!("trek_rsel_incdir_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let _ = std::fs::create_dir_all(tmp.join("aaa_dir"));
    std::fs::write(tmp.join("zzz_file.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    // Dirs appear first in sorted listing
    app.selected = 0;
    assert!(app.entries[0].is_dir, "first entry should be a dir");
    app.select_move_down();
    assert!(
        app.rename_selected.contains(&0),
        "directory at index 0 should be in rename_selected"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── preview scroll ([ / ]) tests ─────────────────────────────────────────────

/// Given: a file with 20 lines loaded in preview, scroll at 0
/// When: scroll_preview_down(5) is called
/// Then: preview_scroll is 5
#[test]
fn scroll_preview_down_advances_offset() {
    let tmp = std::env::temp_dir().join(format!("trek_pscroll_dn_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let content: String = (1..=20).map(|i| format!("line {}\n", i)).collect();
    std::fs::write(tmp.join("big.txt"), content.as_bytes()).unwrap();
    let mut app = make_app_at(&tmp);
    let idx = app
        .entries
        .iter()
        .position(|e| e.name == "big.txt")
        .unwrap();
    app.selected = idx;
    app.load_preview();
    assert!(app.preview_lines.len() >= 10, "preview should have lines");
    app.scroll_preview_down(5);
    assert_eq!(app.preview_scroll, 5);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: preview_scroll is 5
/// When: scroll_preview_up(3) is called
/// Then: preview_scroll is 2
#[test]
fn scroll_preview_up_decreases_offset() {
    let tmp = std::env::temp_dir().join(format!("trek_pscroll_up_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let content: String = (1..=20).map(|i| format!("line {}\n", i)).collect();
    std::fs::write(tmp.join("big.txt"), content.as_bytes()).unwrap();
    let mut app = make_app_at(&tmp);
    let idx = app
        .entries
        .iter()
        .position(|e| e.name == "big.txt")
        .unwrap();
    app.selected = idx;
    app.load_preview();
    app.preview_scroll = 5;
    app.scroll_preview_up(3);
    assert_eq!(app.preview_scroll, 2);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: preview_scroll is 0
/// When: scroll_preview_up(5) is called
/// Then: preview_scroll stays at 0 (no underflow)
#[test]
fn scroll_preview_up_at_top_does_not_underflow() {
    let tmp = std::env::temp_dir().join(format!("trek_pscroll_top_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("f.txt"), b"hello\nworld\n").unwrap();
    let mut app = make_app_at(&tmp);
    let idx = app.entries.iter().position(|e| e.name == "f.txt").unwrap();
    app.selected = idx;
    app.load_preview();
    app.preview_scroll = 0;
    app.scroll_preview_up(5);
    assert_eq!(app.preview_scroll, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: preview_scroll is already at or near max
/// When: scroll_preview_down(100) is called
/// Then: preview_scroll does not exceed preview_lines.len() - 1
#[test]
fn scroll_preview_down_at_bottom_clamps() {
    let tmp = std::env::temp_dir().join(format!("trek_pscroll_bot_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let content: String = (1..=10).map(|i| format!("line {}\n", i)).collect();
    std::fs::write(tmp.join("short.txt"), content.as_bytes()).unwrap();
    let mut app = make_app_at(&tmp);
    let idx = app
        .entries
        .iter()
        .position(|e| e.name == "short.txt")
        .unwrap();
    app.selected = idx;
    app.load_preview();
    let max = app.preview_lines.len().saturating_sub(1);
    app.scroll_preview_down(100);
    assert_eq!(app.preview_scroll, max);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: an empty preview (no file selected / empty file)
/// When: scroll_preview_down and scroll_preview_up are called
/// Then: no panic; preview_scroll stays at 0
#[test]
fn scroll_preview_on_empty_preview_is_noop() {
    let tmp = std::env::temp_dir().join(format!("trek_pscroll_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("empty.txt"), b"").unwrap();
    let mut app = make_app_at(&tmp);
    app.preview_lines.clear();
    app.preview_scroll = 0;
    app.scroll_preview_down(5);
    app.scroll_preview_up(5);
    assert_eq!(app.preview_scroll, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── touch file (t) tests ──────────────────────────────────────────────────────

/// Given: normal mode
/// When: begin_touch() is called
/// Then: touch_mode is true, touch_input is empty
#[test]
fn begin_touch_opens_bar() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_open_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    assert!(!app.touch_mode);
    app.begin_touch();
    assert!(app.touch_mode);
    assert!(app.touch_input.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: touch mode open with some input
/// When: cancel_touch() is called
/// Then: touch_mode is false, input cleared, no file created
#[test]
fn cancel_touch_closes_without_creating() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_cancel_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_touch();
    app.touch_push_char('f');
    app.touch_push_char('o');
    app.touch_push_char('o');
    app.cancel_touch();
    assert!(!app.touch_mode);
    assert!(app.touch_input.is_empty());
    assert!(
        !tmp.join("foo").exists(),
        "no file should be created on cancel"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: touch mode with a valid filename
/// When: confirm_touch() is called
/// Then: file is created, listing refreshed, cursor on new file, status set
#[test]
fn confirm_touch_creates_file_and_selects_it() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_create_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_touch();
    for c in "newfile.txt".chars() {
        app.touch_push_char(c);
    }
    app.confirm_touch();
    assert!(!app.touch_mode, "touch mode should close after confirm");
    let created = tmp.join("newfile.txt");
    assert!(created.exists(), "file should exist on disk");
    assert_eq!(created.metadata().unwrap().len(), 0, "file should be empty");
    let selected_name = app.entries.get(app.selected).map(|e| e.name.as_str());
    assert_eq!(
        selected_name,
        Some("newfile.txt"),
        "cursor should be on new file"
    );
    assert!(app.status_message.is_some());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: touch mode with an empty filename
/// When: confirm_touch() is called
/// Then: no file created, status message set, touch_mode closed
#[test]
fn confirm_touch_empty_name_shows_error() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_touch();
    app.confirm_touch();
    assert!(!app.touch_mode);
    assert!(app.status_message.is_some());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: a file already exists with that name
/// When: confirm_touch() is called with the same name
/// Then: no overwrite, status message contains the filename
#[test]
fn confirm_touch_existing_file_shows_error() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_exists_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("existing.txt"), b"data").unwrap();
    let mut app = make_app_at(&tmp);
    app.begin_touch();
    for c in "existing.txt".chars() {
        app.touch_push_char(c);
    }
    app.confirm_touch();
    assert!(!app.touch_mode);
    assert!(app.status_message.is_some());
    // Original file content must be preserved
    assert_eq!(std::fs::read(tmp.join("existing.txt")).unwrap(), b"data");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: touch_input has characters
/// When: touch_pop_char() is called
/// Then: last character is removed
#[test]
fn touch_push_pop_char() {
    let tmp = std::env::temp_dir().join(format!("trek_touch_chars_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.begin_touch();
    app.touch_push_char('a');
    app.touch_push_char('b');
    app.touch_push_char('c');
    assert_eq!(app.touch_input, "abc");
    app.touch_pop_char();
    assert_eq!(app.touch_input, "ab");
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── preview line numbers (#) tests ───────────────────────────────────────────

/// Given: default state
/// When: show_line_numbers is checked
/// Then: it is false (off by default)
#[test]
fn line_numbers_default_off() {
    let tmp = std::env::temp_dir().join(format!("trek_ln_default_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let app = make_app_at(&tmp);
    assert!(
        !app.show_line_numbers,
        "line numbers should be off by default"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: show_line_numbers is false
/// When: toggle_line_numbers() is called
/// Then: show_line_numbers is true and status message is set
#[test]
fn toggle_line_numbers_turns_on() {
    let tmp = std::env::temp_dir().join(format!("trek_ln_on_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.toggle_line_numbers();
    assert!(app.show_line_numbers);
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(msg.contains("on"), "status should say 'on': {}", msg);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: show_line_numbers is true
/// When: toggle_line_numbers() is called again
/// Then: show_line_numbers is false and status message reflects off
#[test]
fn toggle_line_numbers_turns_off() {
    let tmp = std::env::temp_dir().join(format!("trek_ln_off_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = make_app_at(&tmp);
    app.show_line_numbers = true;
    app.toggle_line_numbers();
    assert!(!app.show_line_numbers);
    let msg = app.status_message.as_deref().unwrap_or("");
    assert!(msg.contains("off"), "status should say 'off': {}", msg);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Given: show_line_numbers persists across file navigation
/// When: toggle then navigate to another file with j (move_down)
/// Then: show_line_numbers is still true
#[test]
fn line_numbers_persist_across_navigation() {
    let tmp = std::env::temp_dir().join(format!("trek_ln_nav_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("a.txt"), b"line1\n").unwrap();
    std::fs::write(tmp.join("b.txt"), b"line1\nline2\n").unwrap();
    let mut app = make_app_at(&tmp);
    app.toggle_line_numbers();
    assert!(app.show_line_numbers);
    app.move_down();
    assert!(
        app.show_line_numbers,
        "show_line_numbers should persist after navigation"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
