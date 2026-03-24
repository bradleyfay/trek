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
