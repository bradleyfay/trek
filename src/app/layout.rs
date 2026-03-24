use super::App;

impl App {
    /// Store computed layout values needed for mouse hit-testing.
    /// Called once per frame by `ui::draw` after it calculates pane geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_layout(
        &mut self,
        term_width: u16,
        term_height: u16,
        left_div_col: u16,
        right_div_col: u16,
        parent_area: (u16, u16, u16, u16),
        current_area: (u16, u16, u16, u16),
        preview_area: (u16, u16, u16, u16),
    ) {
        self.term_width = term_width;
        self.term_height = term_height;
        self.left_div_col = left_div_col;
        self.right_div_col = right_div_col;
        self.parent_area = parent_area;
        self.current_area = current_area;
        self.preview_area = preview_area;
    }

    pub fn is_in_rect(&self, col: u16, row: u16, area: (u16, u16, u16, u16)) -> bool {
        let (x, y, w, h) = area;
        col >= x && col < x + w && row >= y && row < y + h
    }

    pub fn is_in_preview(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.preview_area)
    }

    pub fn is_in_current(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.current_area)
    }

    pub fn is_in_parent(&self, col: u16, row: u16) -> bool {
        self.is_in_rect(col, row, self.parent_area)
    }
}
