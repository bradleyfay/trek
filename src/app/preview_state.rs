pub struct PreviewState {
    /// Lines of the previewed file (right pane).
    pub preview_lines: Vec<String>,
    /// Preview scroll offset (line index of top visible line).
    pub preview_scroll: usize,
    /// True while a background thread is rendering the preview.
    pub preview_loading: bool,
    /// Receive end of the async preview channel.
    pub preview_rx: Option<std::sync::mpsc::Receiver<crate::app::preview::PreviewResult>>,

    // --- Preview pane collapse (w) ---
    /// True when the right preview pane is collapsed (hidden).
    pub preview_collapsed: bool,
    /// Saved `right_div` ratio restored when expanding the pane.
    pub preview_right_div: f64,

    // --- Git diff preview ---
    /// True when `preview_lines` holds diff output (used by the renderer to colorise lines).
    pub preview_is_diff: bool,

    // --- Special preview modes ---
    /// When true the preview pane shows `git diff` output instead of raw file content.
    pub diff_preview_mode: bool,
    /// When true the preview pane shows the file metadata card instead of content.
    pub meta_preview_mode: bool,
    /// When true the preview pane shows `git log --oneline -30 -- <path>`.
    pub git_log_mode: bool,
    /// When true the preview pane shows a unified diff of the two selected files.
    pub file_compare_mode: bool,
    /// When true the preview pane shows a hex dump (xxd / hexdump -C).
    pub hex_view_mode: bool,
    /// When true the preview pane shows a `du -k -d 1` breakdown of the selected directory.
    pub du_preview_mode: bool,

    // --- Preview line numbers (#) ---
    pub show_line_numbers: bool,
    /// When true, the preview pane soft-wraps long lines at the pane boundary.
    pub preview_wrap: bool,

    // --- Preview focus mode (Right / l on a file) ---
    /// True when the cursor has moved into the preview pane (focus mode active).
    pub preview_focused: bool,
    /// Absolute line index of the cursor within preview_lines.
    pub preview_cursor: usize,
    /// Start of a selection range; None means no active range selection.
    pub preview_selection_anchor: Option<usize>,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            preview_lines: Vec::new(),
            preview_scroll: 0,
            preview_loading: false,
            preview_rx: None,
            preview_collapsed: false,
            preview_right_div: 0.55,
            preview_is_diff: false,
            diff_preview_mode: false,
            meta_preview_mode: false,
            git_log_mode: false,
            file_compare_mode: false,
            hex_view_mode: false,
            du_preview_mode: false,
            show_line_numbers: false,
            preview_wrap: false,
            preview_focused: false,
            preview_cursor: 0,
            preview_selection_anchor: None,
        }
    }
}
