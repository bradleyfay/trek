use super::{palette, App};

impl App {
    /// Open the command palette: reset query, rebuild filtered list, show overlay.
    pub fn open_palette(&mut self) {
        self.overlay.palette_mode = true;
        self.overlay.palette_query.clear();
        self.overlay.palette_filtered = palette::filter_palette("");
        self.overlay.palette_selected = 0;
    }

    /// Close the command palette without executing any action.
    pub fn close_palette(&mut self) {
        self.overlay.palette_mode = false;
        self.overlay.palette_query.clear();
    }

    /// Append a character to the palette query and re-filter.
    pub fn palette_push_char(&mut self, c: char) {
        self.overlay.palette_query.push(c);
        self.overlay.palette_filtered = palette::filter_palette(&self.overlay.palette_query);
        self.overlay.palette_selected = 0;
    }

    /// Remove the last character from the palette query and re-filter.
    pub fn palette_pop_char(&mut self) {
        self.overlay.palette_query.pop();
        self.overlay.palette_filtered = palette::filter_palette(&self.overlay.palette_query);
        self.overlay.palette_selected = 0;
    }

    /// Move the palette cursor down one row, clamped to the last result.
    pub fn palette_move_down(&mut self) {
        if !self.overlay.palette_filtered.is_empty() {
            self.overlay.palette_selected =
                (self.overlay.palette_selected + 1).min(self.overlay.palette_filtered.len() - 1);
        }
    }

    /// Move the palette cursor up one row, clamped to 0.
    pub fn palette_move_up(&mut self) {
        self.overlay.palette_selected = self.overlay.palette_selected.saturating_sub(1);
    }

    /// Return the ActionId of the currently highlighted palette row, if any.
    pub fn palette_selected_action(&self) -> Option<palette::ActionId> {
        self.overlay
            .palette_filtered
            .get(self.overlay.palette_selected)
            .and_then(|&i| palette::PALETTE_ACTIONS.get(i))
            .map(|a| a.id)
    }
}
