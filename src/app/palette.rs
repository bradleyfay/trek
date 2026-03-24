/// Command palette action registry.
///
/// Every Trek command is registered here with a human-readable name and
/// keybinding hint. When adding a new feature, add a corresponding entry to
/// PALETTE_ACTIONS so it is discoverable from the palette.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    GoHome,
    GoTop,
    GoBottom,
    HistoryBack,
    HistoryForward,
    BeginSetMark,
    BeginJumpMark,
    ToggleHidden,
    ToggleGitignored,
    ToggleDiffPreview,
    ToggleMetaPreview,
    ToggleHashPreview,
    TogglePreviewPane,
    ToggleTimestamps,
    RefreshGitStatus,
    StartSearch,
    StartFilter,
    StartContentSearch,
    StartFind,
    ClipboardCopyCurrent,
    ClipboardCopySelected,
    ClipboardCutCurrent,
    PasteClipboard,
    BeginDeleteCurrent,
    BeginDeleteSelected,
    BeginMkdir,
    BeginTouch,
    UndoTrash,
    BeginChmod,
    SelectMoveDown,
    SelectMoveUp,
    ToggleSelection,
    SelectAll,
    ClearSelections,
    QuickRename,
    StartRename,
    AddBookmark,
    OpenBookmarks,
    CycleSortMode,
    ToggleSortOrder,
    YankRelativePath,
    YankAbsolutePath,
    OpenYankPicker,
    ToggleLineNumbers,
    ScrollPreviewUp,
    ScrollPreviewDown,
    PathJump,
    GlobSelect,
    BeginDup,
    BeginSymlink,
    ShowHelp,
    Quit,
}

#[derive(Clone, Copy, Debug)]
pub struct PaletteAction {
    pub id: ActionId,
    /// Searched when filtering — full English description.
    pub name: &'static str,
    /// Displayed alongside the action name in the overlay.
    pub keys: &'static str,
}

pub static PALETTE_ACTIONS: &[PaletteAction] = &[
    PaletteAction {
        id: ActionId::GoHome,
        name: "Go to home directory",
        keys: "~",
    },
    PaletteAction {
        id: ActionId::GoTop,
        name: "Go to top of list",
        keys: "g",
    },
    PaletteAction {
        id: ActionId::GoBottom,
        name: "Go to bottom of list",
        keys: "G",
    },
    PaletteAction {
        id: ActionId::HistoryBack,
        name: "Go back in history",
        keys: "Ctrl+O",
    },
    PaletteAction {
        id: ActionId::HistoryForward,
        name: "Go forward in history",
        keys: "Ctrl+I",
    },
    PaletteAction {
        id: ActionId::BeginSetMark,
        name: "Set mark (record current directory to a letter slot)",
        keys: "` <letter>",
    },
    PaletteAction {
        id: ActionId::BeginJumpMark,
        name: "Jump to mark (navigate to a previously marked directory)",
        keys: "' <letter>",
    },
    PaletteAction {
        id: ActionId::ToggleHidden,
        name: "Toggle hidden files",
        keys: ".",
    },
    PaletteAction {
        id: ActionId::ToggleGitignored,
        name: "Toggle gitignore filter (hide ignored files)",
        keys: "i",
    },
    PaletteAction {
        id: ActionId::ToggleDiffPreview,
        name: "Toggle diff preview",
        keys: "d",
    },
    PaletteAction {
        id: ActionId::ToggleMetaPreview,
        name: "Toggle meta preview (permissions, size)",
        keys: "m",
    },
    PaletteAction {
        id: ActionId::ToggleHashPreview,
        name: "Toggle hash preview (SHA-256 checksum)",
        keys: "H",
    },
    PaletteAction {
        id: ActionId::TogglePreviewPane,
        name: "Toggle preview pane (hide/show right pane)",
        keys: "w",
    },
    PaletteAction {
        id: ActionId::ToggleTimestamps,
        name: "Toggle modification timestamps in listing",
        keys: "T",
    },
    PaletteAction {
        id: ActionId::RefreshGitStatus,
        name: "Refresh git status",
        keys: "R",
    },
    PaletteAction {
        id: ActionId::StartSearch,
        name: "Fuzzy search",
        keys: "/",
    },
    PaletteAction {
        id: ActionId::StartFilter,
        name: "Filter / narrow listing",
        keys: "|",
    },
    PaletteAction {
        id: ActionId::StartContentSearch,
        name: "Content search (ripgrep)",
        keys: "Ctrl+F",
    },
    PaletteAction {
        id: ActionId::StartFind,
        name: "Recursive filename find",
        keys: "Ctrl+P",
    },
    PaletteAction {
        id: ActionId::ClipboardCopyCurrent,
        name: "Copy current file to clipboard",
        keys: "c",
    },
    PaletteAction {
        id: ActionId::ClipboardCopySelected,
        name: "Copy selected files to clipboard",
        keys: "C",
    },
    PaletteAction {
        id: ActionId::ClipboardCutCurrent,
        name: "Cut current file to clipboard",
        keys: "x",
    },
    PaletteAction {
        id: ActionId::PasteClipboard,
        name: "Paste clipboard into current directory",
        keys: "p",
    },
    PaletteAction {
        id: ActionId::BeginDeleteCurrent,
        name: "Trash current file or directory",
        keys: "Delete",
    },
    PaletteAction {
        id: ActionId::BeginDeleteSelected,
        name: "Trash all selected files",
        keys: "X",
    },
    PaletteAction {
        id: ActionId::BeginMkdir,
        name: "New directory",
        keys: "M",
    },
    PaletteAction {
        id: ActionId::BeginTouch,
        name: "New file (touch — create empty file)",
        keys: "t",
    },
    PaletteAction {
        id: ActionId::UndoTrash,
        name: "Undo last trash operation",
        keys: "u",
    },
    PaletteAction {
        id: ActionId::BeginChmod,
        name: "chmod — edit file permissions",
        keys: "P",
    },
    PaletteAction {
        id: ActionId::SelectMoveDown,
        name: "Extend selection down (range select)",
        keys: "J",
    },
    PaletteAction {
        id: ActionId::SelectMoveUp,
        name: "Extend selection up (range select)",
        keys: "K",
    },
    PaletteAction {
        id: ActionId::ToggleSelection,
        name: "Toggle file selection",
        keys: "Space",
    },
    PaletteAction {
        id: ActionId::SelectAll,
        name: "Select all files",
        keys: "v",
    },
    PaletteAction {
        id: ActionId::ClearSelections,
        name: "Clear selections",
        keys: "Esc",
    },
    PaletteAction {
        id: ActionId::QuickRename,
        name: "Quick rename current file or directory",
        keys: "n / F2",
    },
    PaletteAction {
        id: ActionId::StartRename,
        name: "Bulk rename with regex",
        keys: "r",
    },
    PaletteAction {
        id: ActionId::AddBookmark,
        name: "Add bookmark for current directory",
        keys: "b",
    },
    PaletteAction {
        id: ActionId::OpenBookmarks,
        name: "Open bookmark picker",
        keys: "B",
    },
    PaletteAction {
        id: ActionId::CycleSortMode,
        name: "Cycle sort mode (name/size/modified/ext)",
        keys: "S",
    },
    PaletteAction {
        id: ActionId::ToggleSortOrder,
        name: "Toggle sort order (asc/desc)",
        keys: "s",
    },
    PaletteAction {
        id: ActionId::YankRelativePath,
        name: "Yank relative path to clipboard",
        keys: "y",
    },
    PaletteAction {
        id: ActionId::YankAbsolutePath,
        name: "Yank absolute path to clipboard",
        keys: "Y",
    },
    PaletteAction {
        id: ActionId::OpenYankPicker,
        name: "Yank path (pick format: relative/absolute/filename/parent)",
        keys: "A",
    },
    PaletteAction {
        id: ActionId::ToggleLineNumbers,
        name: "Toggle line numbers in preview pane",
        keys: "#",
    },
    PaletteAction {
        id: ActionId::ScrollPreviewUp,
        name: "Scroll preview pane up",
        keys: "[",
    },
    PaletteAction {
        id: ActionId::ScrollPreviewDown,
        name: "Scroll preview pane down",
        keys: "]",
    },
    PaletteAction {
        id: ActionId::PathJump,
        name: "Jump to path (path jump bar)",
        keys: "e",
    },
    PaletteAction {
        id: ActionId::GlobSelect,
        name: "Select files by glob pattern",
        keys: "*",
    },
    PaletteAction {
        id: ActionId::BeginDup,
        name: "Duplicate entry in place",
        keys: "W",
    },
    PaletteAction {
        id: ActionId::BeginSymlink,
        name: "Create symlink to selected entry",
        keys: "L",
    },
    PaletteAction {
        id: ActionId::ShowHelp,
        name: "Show help overlay",
        keys: "?",
    },
    // Quit appears for discoverability; palette dispatch treats it as a no-op
    // because the event loop break cannot be triggered from a function call.
    // Users should press q directly to quit.
    PaletteAction {
        id: ActionId::Quit,
        name: "Quit trek",
        keys: "q",
    },
];

/// Return indices into PALETTE_ACTIONS where `query` is a case-insensitive
/// substring of the action name. Empty query returns all indices.
pub fn filter_palette(query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    PALETTE_ACTIONS
        .iter()
        .enumerate()
        .filter(|(_, a)| q.is_empty() || a.name.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect()
}
