use super::{fuzzy_match, App};

impl App {
    pub fn start_search(&mut self) {
        self.nav.search_mode = true;
        self.nav.search_query.clear();
        self.nav.pre_search_selected = self.nav.selected;
        self.update_filter();
    }

    pub fn cancel_search(&mut self) {
        self.nav.selected = self.nav.pre_search_selected;
        self.nav.search_mode = false;
        self.nav.search_query.clear();
        self.nav.filtered_indices.clear();
        self.nav.filtered_set.clear();
        self.load_preview();
    }

    pub fn confirm_search(&mut self) {
        // Move selection to the first filtered match, then exit search mode.
        if let Some(&idx) = self.nav.filtered_indices.first() {
            self.nav.selected = idx;
            self.load_preview();
        }
        self.nav.search_mode = false;
        self.nav.search_query.clear();
        self.nav.filtered_indices.clear();
        self.nav.filtered_set.clear();
    }

    pub fn search_push_char(&mut self, c: char) {
        self.nav.search_query.push(c);
        self.update_filter();
    }

    pub fn search_pop_char(&mut self) {
        self.nav.search_query.pop();
        self.update_filter();
    }

    pub fn search_move_down(&mut self) {
        // Move to the next filtered match after current selection.
        if let Some(pos) = self
            .nav
            .filtered_indices
            .iter()
            .position(|&i| i > self.nav.selected)
        {
            self.nav.selected = self.nav.filtered_indices[pos];
            self.load_preview();
        }
    }

    pub fn search_move_up(&mut self) {
        // Move to the previous filtered match before current selection.
        if let Some(pos) = self
            .nav
            .filtered_indices
            .iter()
            .rposition(|&i| i < self.nav.selected)
        {
            self.nav.selected = self.nav.filtered_indices[pos];
            self.load_preview();
        }
    }

    fn update_filter(&mut self) {
        if self.nav.search_query.is_empty() {
            self.nav.filtered_indices = (0..self.nav.entries.len()).collect();
        } else {
            let query = self.nav.search_query.to_lowercase();
            self.nav.filtered_indices = self
                .nav
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| fuzzy_match(&e.name.to_lowercase(), &query))
                .map(|(i, _)| i)
                .collect();
        }
        self.nav.filtered_set = self.nav.filtered_indices.iter().copied().collect();
        // Auto-select first match.
        if let Some(&first) = self.nav.filtered_indices.first() {
            self.nav.selected = first;
            self.load_preview();
        }
    }
}
