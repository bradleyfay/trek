use super::App;

impl App {
    /// Open the filter input bar.
    pub fn start_filter(&mut self) {
        self.filter_mode = true;
    }

    /// Close the bar but keep the filter active ("frozen" state).
    pub fn close_filter(&mut self) {
        self.filter_mode = false;
    }

    /// Clear the active filter and restore the full listing.
    pub fn clear_filter(&mut self) {
        self.filter_input.clear();
        self.filter_mode = false;
        self.current_scroll = 0;
        self.load_dir();
    }

    /// Add a character to the filter and re-narrow the listing.
    pub fn filter_push_char(&mut self, c: char) {
        self.filter_input.push(c);
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
    }

    /// Remove the last character from the filter and reload.
    pub fn filter_pop_char(&mut self) {
        self.filter_input.pop();
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
    }
}
