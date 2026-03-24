use super::{palette, App};

impl App {
    /// Open the command palette: reset query, rebuild filtered list, show overlay.
    pub fn open_palette(&mut self) {
        self.palette_mode = true;
        self.palette_query.clear();
        self.palette_filtered = palette::filter_palette("");
        self.palette_selected = 0;
    }

    /// Close the command palette without executing any action.
    pub fn close_palette(&mut self) {
        self.palette_mode = false;
        self.palette_query.clear();
    }

    /// Append a character to the palette query and re-filter.
    pub fn palette_push_char(&mut self, c: char) {
        self.palette_query.push(c);
        self.palette_filtered = palette::filter_palette(&self.palette_query);
        self.palette_selected = 0;
    }

    /// Remove the last character from the palette query and re-filter.
    pub fn palette_pop_char(&mut self) {
        self.palette_query.pop();
        self.palette_filtered = palette::filter_palette(&self.palette_query);
        self.palette_selected = 0;
    }

    /// Move the palette cursor down one row, clamped to the last result.
    pub fn palette_move_down(&mut self) {
        if !self.palette_filtered.is_empty() {
            self.palette_selected =
                (self.palette_selected + 1).min(self.palette_filtered.len() - 1);
        }
    }

    /// Move the palette cursor up one row, clamped to 0.
    pub fn palette_move_up(&mut self) {
        self.palette_selected = self.palette_selected.saturating_sub(1);
    }

    /// Return the ActionId of the currently highlighted palette row, if any.
    pub fn palette_selected_action(&self) -> Option<palette::ActionId> {
        self.palette_filtered
            .get(self.palette_selected)
            .and_then(|&i| palette::PALETTE_ACTIONS.get(i))
            .map(|a| a.id)
    }
}
