/// Trek colour theme.
///
/// Every colour used in the TUI is expressed through a named semantic role
/// here rather than as an inline `Color::X` literal.  Adding a new theme
/// means providing a new `Theme` value — no render code needs to change.
///
/// # Available themes
///
/// | Name                 | Style |
/// |----------------------|-------|
/// | `default`            | dark  |
/// | `catppuccin-mocha`   | dark  |
/// | `catppuccin-latte`   | light |
/// | `tokyo-night`        | dark  |
/// | `tokyo-night-light`  | light |
use ratatui::style::Color;

/// A complete set of semantic colours for the Trek TUI.
#[derive(Clone, Debug)]
pub struct Theme {
    // ── Core text ─────────────────────────────────────────────────────────────
    /// Primary text — filenames, titles, input content.
    pub fg: Color,
    /// Dimmed/secondary text — hints, sizes, meta info, inactive labels.
    pub fg_dim: Color,

    // ── Cursor / list selection ───────────────────────────────────────────────
    /// Foreground on the highlighted cursor row.
    pub sel_fg: Color,
    /// Background of the highlighted cursor row.
    pub sel_bg: Color,
    /// Background of subtly-highlighted rows (task-manager, non-focused).
    pub subtle_sel_bg: Color,

    // ── File types ────────────────────────────────────────────────────────────
    /// Directory name colour.
    pub dir_fg: Color,

    // ── Git status indicators ─────────────────────────────────────────────────
    /// Modified (●).
    pub git_modified: Color,
    /// Staged / staged-modified (✚).
    pub git_staged: Color,
    /// Untracked (+).
    pub git_untracked: Color,
    /// Deleted / conflict (✖).
    pub git_deleted: Color,

    // ── Multi-selection ───────────────────────────────────────────────────────
    /// Colour of selected-but-not-cursor entry text.
    pub multi_sel_fg: Color,
    /// Colour of the ✓ checkmark prefix.
    pub multi_sel_mark: Color,

    // ── Borders ───────────────────────────────────────────────────────────────
    /// Inactive / background pane borders.
    pub border: Color,
    /// Active / focused pane border (current pane, focused preview, overlays).
    pub border_focus: Color,
    /// Warning-accented border (task manager, frecency, yank picker).
    pub border_warn: Color,

    // ── Input bars ────────────────────────────────────────────────────────────
    /// Label prefix for primary input bars ("Rename:", "Find:", etc.).
    pub prompt: Color,
    /// Label prefix for secondary input bars ("Jump to:", "New file:", etc.).
    pub prompt_alt: Color,
    /// Colour of user-typed text inside input bars.
    pub input: Color,
    /// Colour of the block-cursor character (█).
    pub cursor: Color,

    // ── Semantic status ───────────────────────────────────────────────────────
    /// Success / positive feedback (green family).
    pub ok: Color,
    /// Error / destructive action (red family).
    pub error: Color,
    /// Warning / caution (yellow/orange family).
    pub warn: Color,
    /// Informational / neutral accent (cyan/blue family).
    pub info: Color,

    // ── Confirm badge (Extract bar) ───────────────────────────────────────────
    /// Foreground text on the coloured confirm badge.
    pub confirm_fg: Color,
    /// Background of the coloured confirm badge.
    pub confirm_bg: Color,

    // ── Diff colours ─────────────────────────────────────────────────────────
    pub diff_add: Color,
    pub diff_del: Color,
    pub diff_hunk: Color,
    pub diff_meta: Color,

    // ── Change-feed / session-summary event types ─────────────────────────────
    pub event_new: Color,
    pub event_modified: Color,
    pub event_deleted: Color,

    // ── Scrollbar ─────────────────────────────────────────────────────────────
    pub scrollbar_thumb: Color,
    pub scrollbar_track: Color,

    // ── Syntax highlighting ───────────────────────────────────────────────────
    /// Name of the `syntect` theme used for source-file preview highlighting.
    /// Must be a key present in `ThemeSet::load_defaults()`.
    pub syntax_theme: &'static str,
}

/// Single source of truth mapping CLI names to constructors.
/// Both `Theme::from_name` and `Theme::names` are derived from this table,
/// so adding a new theme means adding one entry here and writing the constructor.
type ThemeEntry = (&'static str, fn() -> Theme);
const REGISTRY: &[ThemeEntry] = &[
    ("default", Theme::default),
    ("catppuccin-mocha", Theme::catppuccin_mocha),
    ("catppuccin-latte", Theme::catppuccin_latte),
    ("tokyo-night", Theme::tokyo_night),
    ("tokyo-night-light", Theme::tokyo_night_light),
];

impl Theme {
    /// Resolve a theme by name.  Returns `None` for unrecognised names so the
    /// caller can report a proper error without calling `process::exit` inside
    /// the theme module.
    pub fn from_name(name: &str) -> Option<Self> {
        REGISTRY
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, constructor)| constructor())
    }

    /// Returns an iterator over all recognised theme names.
    pub fn names() -> impl Iterator<Item = &'static str> {
        REGISTRY.iter().map(|(name, _)| *name)
    }

    // ── Built-in themes ───────────────────────────────────────────────────────

    /// Trek's original dark colour scheme — a clean set of ANSI named colours
    /// that work in virtually every terminal.
    pub fn default() -> Self {
        Self {
            fg: Color::White,
            fg_dim: Color::DarkGray,
            sel_fg: Color::White,
            sel_bg: Color::Blue,
            subtle_sel_bg: Color::DarkGray,
            dir_fg: Color::Cyan,
            git_modified: Color::Yellow,
            git_staged: Color::Green,
            git_untracked: Color::Cyan,
            git_deleted: Color::Red,
            multi_sel_fg: Color::Magenta,
            multi_sel_mark: Color::Green,
            border: Color::DarkGray,
            border_focus: Color::Cyan,
            border_warn: Color::Yellow,
            prompt: Color::Yellow,
            prompt_alt: Color::Cyan,
            input: Color::White,
            cursor: Color::White,
            ok: Color::Green,
            error: Color::Red,
            warn: Color::Yellow,
            info: Color::Cyan,
            confirm_fg: Color::Black,
            confirm_bg: Color::LightGreen,
            diff_add: Color::Green,
            diff_del: Color::Red,
            diff_hunk: Color::Cyan,
            diff_meta: Color::DarkGray,
            event_new: Color::Green,
            event_modified: Color::Yellow,
            event_deleted: Color::Red,
            scrollbar_thumb: Color::Gray,
            scrollbar_track: Color::Rgb(60, 60, 60),
            syntax_theme: "base16-ocean.dark",
        }
    }

    /// Catppuccin Mocha — a warm, dark theme with pastel accents.
    /// <https://github.com/catppuccin/catppuccin>
    pub fn catppuccin_mocha() -> Self {
        // Palette (Mocha flavour)
        const TEXT: Color = Color::Rgb(205, 214, 244); // #cdd6f4
        const OVERLAY0: Color = Color::Rgb(108, 112, 134); // #6c7086
        const BASE: Color = Color::Rgb(30, 30, 46); // #1e1e2e
        const SURFACE0: Color = Color::Rgb(49, 50, 68); // #313244
        const SURFACE1: Color = Color::Rgb(69, 71, 90); // #45475a
        const SURFACE2: Color = Color::Rgb(88, 91, 112); // #585b70
        const BLUE: Color = Color::Rgb(137, 180, 250); // #89b4fa
        const SAPPHIRE: Color = Color::Rgb(116, 199, 236); // #74c7ec
        const SKY: Color = Color::Rgb(137, 220, 235); // #89dceb
        const GREEN: Color = Color::Rgb(166, 227, 161); // #a6e3a1
        const YELLOW: Color = Color::Rgb(249, 226, 175); // #f9e2af
        const RED: Color = Color::Rgb(243, 139, 168); // #f38ba8
        const MAUVE: Color = Color::Rgb(203, 166, 247); // #cba6f7
        const FLAMINGO: Color = Color::Rgb(242, 205, 205); // #f2cdcd

        Self {
            fg: TEXT,
            fg_dim: OVERLAY0,
            sel_fg: BASE,
            sel_bg: BLUE,
            subtle_sel_bg: SURFACE0,
            dir_fg: SAPPHIRE,
            git_modified: YELLOW,
            git_staged: GREEN,
            git_untracked: SKY,
            git_deleted: RED,
            multi_sel_fg: MAUVE,
            multi_sel_mark: GREEN,
            border: SURFACE1,
            border_focus: BLUE,
            border_warn: YELLOW,
            prompt: YELLOW,
            prompt_alt: SKY,
            input: TEXT,
            cursor: FLAMINGO,
            ok: GREEN,
            error: RED,
            warn: YELLOW,
            info: SKY,
            confirm_fg: BASE,
            confirm_bg: GREEN,
            diff_add: GREEN,
            diff_del: RED,
            diff_hunk: BLUE,
            diff_meta: OVERLAY0,
            event_new: GREEN,
            event_modified: YELLOW,
            event_deleted: RED,
            scrollbar_thumb: SURFACE2,
            scrollbar_track: SURFACE0,
            syntax_theme: "base16-ocean.dark",
        }
    }

    /// Catppuccin Latte — a soft, light theme with warm pastel accents.
    /// <https://github.com/catppuccin/catppuccin>
    pub fn catppuccin_latte() -> Self {
        // Palette (Latte flavour)
        const TEXT: Color = Color::Rgb(76, 79, 105); // #4c4f69
        const OVERLAY0: Color = Color::Rgb(156, 160, 176); // #9ca0b0
        const BASE: Color = Color::Rgb(239, 241, 245); // #eff1f5
        const SURFACE1: Color = Color::Rgb(204, 208, 218); // #ccd0da
        const BLUE: Color = Color::Rgb(30, 102, 245); // #1e66f5
        const SAPPHIRE: Color = Color::Rgb(32, 159, 181); // #209fb5
        const SKY: Color = Color::Rgb(4, 165, 229); // #04a5e5
        const GREEN: Color = Color::Rgb(64, 160, 43); // #40a02b
        const YELLOW: Color = Color::Rgb(223, 142, 29); // #df8e1d
        const RED: Color = Color::Rgb(210, 15, 57); // #d20f39
        const MAUVE: Color = Color::Rgb(136, 57, 239); // #8839ef
        const FLAMINGO: Color = Color::Rgb(221, 120, 120); // #dd7878

        Self {
            fg: TEXT,
            fg_dim: OVERLAY0,
            sel_fg: BASE,
            sel_bg: BLUE,
            subtle_sel_bg: SURFACE1,
            dir_fg: SAPPHIRE,
            git_modified: YELLOW,
            git_staged: GREEN,
            git_untracked: SKY,
            git_deleted: RED,
            multi_sel_fg: MAUVE,
            multi_sel_mark: GREEN,
            border: SURFACE1,
            border_focus: BLUE,
            border_warn: YELLOW,
            prompt: YELLOW,
            prompt_alt: SAPPHIRE,
            input: TEXT,
            cursor: FLAMINGO,
            ok: GREEN,
            error: RED,
            warn: YELLOW,
            info: SKY,
            confirm_fg: BASE,
            confirm_bg: GREEN,
            diff_add: GREEN,
            diff_del: RED,
            diff_hunk: BLUE,
            diff_meta: OVERLAY0,
            event_new: GREEN,
            event_modified: YELLOW,
            event_deleted: RED,
            scrollbar_thumb: OVERLAY0,
            scrollbar_track: SURFACE1,
            syntax_theme: "InspiredGitHub",
        }
    }

    /// Tokyo Night — a cool, dark theme inspired by the city at night.
    /// <https://github.com/folke/tokyonight.nvim>
    pub fn tokyo_night() -> Self {
        const FG: Color = Color::Rgb(192, 202, 245); // #c0caf5
        const COMMENT: Color = Color::Rgb(86, 95, 137); // #565f89
        const BG: Color = Color::Rgb(26, 27, 38); // #1a1b26
        const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66); // #292e42
        const TERMINAL_BLACK: Color = Color::Rgb(65, 72, 104); // #414868
        const BLUE: Color = Color::Rgb(122, 162, 247); // #7aa2f7
        const CYAN: Color = Color::Rgb(125, 207, 255); // #7dcfff
        const GREEN: Color = Color::Rgb(158, 206, 106); // #9ece6a
        const YELLOW: Color = Color::Rgb(224, 175, 104); // #e0af68
        const RED: Color = Color::Rgb(247, 118, 142); // #f7768e
        const MAGENTA: Color = Color::Rgb(187, 154, 247); // #bb9af7

        Self {
            fg: FG,
            fg_dim: COMMENT,
            sel_fg: BG,
            sel_bg: BLUE,
            subtle_sel_bg: BG_HIGHLIGHT,
            dir_fg: BLUE,
            git_modified: YELLOW,
            git_staged: GREEN,
            git_untracked: CYAN,
            git_deleted: RED,
            multi_sel_fg: MAGENTA,
            multi_sel_mark: GREEN,
            border: TERMINAL_BLACK,
            border_focus: BLUE,
            border_warn: YELLOW,
            prompt: YELLOW,
            prompt_alt: CYAN,
            input: FG,
            cursor: FG,
            ok: GREEN,
            error: RED,
            warn: YELLOW,
            info: CYAN,
            confirm_fg: BG,
            confirm_bg: GREEN,
            diff_add: GREEN,
            diff_del: RED,
            diff_hunk: BLUE,
            diff_meta: COMMENT,
            event_new: GREEN,
            event_modified: YELLOW,
            event_deleted: RED,
            scrollbar_thumb: TERMINAL_BLACK,
            scrollbar_track: BG_HIGHLIGHT,
            syntax_theme: "base16-ocean.dark",
        }
    }

    /// Tokyo Night Light — the day variant of Tokyo Night.
    /// <https://github.com/folke/tokyonight.nvim>
    pub fn tokyo_night_light() -> Self {
        const FG: Color = Color::Rgb(52, 59, 88); // #343b58
        const COMMENT: Color = Color::Rgb(150, 153, 163); // #9699a3
        const BG: Color = Color::Rgb(213, 214, 219); // #d5d6db
        const BG_HIGHLIGHT: Color = Color::Rgb(196, 200, 218); // #c4c8da
        const BORDER_DIM: Color = Color::Rgb(180, 181, 185); // #b4b5b9
        const BLUE: Color = Color::Rgb(46, 125, 233); // #2e7de9
        const CYAN: Color = Color::Rgb(0, 113, 151); // #007197
        const GREEN: Color = Color::Rgb(88, 117, 57); // #587539
        const YELLOW: Color = Color::Rgb(140, 108, 62); // #8c6c3e
        const RED: Color = Color::Rgb(245, 42, 101); // #f52a65
        const MAGENTA: Color = Color::Rgb(152, 84, 241); // #9854f1

        Self {
            fg: FG,
            fg_dim: COMMENT,
            sel_fg: BG,
            sel_bg: BLUE,
            subtle_sel_bg: BG_HIGHLIGHT,
            dir_fg: BLUE,
            git_modified: YELLOW,
            git_staged: GREEN,
            git_untracked: CYAN,
            git_deleted: RED,
            multi_sel_fg: MAGENTA,
            multi_sel_mark: GREEN,
            border: BORDER_DIM,
            border_focus: BLUE,
            border_warn: YELLOW,
            prompt: YELLOW,
            prompt_alt: CYAN,
            input: FG,
            cursor: FG,
            ok: GREEN,
            error: RED,
            warn: YELLOW,
            info: CYAN,
            confirm_fg: BG,
            confirm_bg: GREEN,
            diff_add: GREEN,
            diff_del: RED,
            diff_hunk: BLUE,
            diff_meta: COMMENT,
            event_new: GREEN,
            event_modified: YELLOW,
            event_deleted: RED,
            scrollbar_thumb: COMMENT,
            scrollbar_track: BG_HIGHLIGHT,
            syntax_theme: "InspiredGitHub",
        }
    }
}
