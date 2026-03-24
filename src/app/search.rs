use super::{fuzzy_match, App};

impl App {
    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.pre_search_selected = self.selected;
        self.update_filter();
    }

    pub fn cancel_search(&mut self) {
        self.selected = self.pre_search_selected;
        self.search_mode = false;
        self.search_query.clear();
        self.filtered_indices.clear();
        self.filtered_set.clear();
        self.load_preview();
    }

    pub fn confirm_search(&mut self) {
        // Move selection to the first filtered match, then exit search mode.
        if let Some(&idx) = self.filtered_indices.first() {
            self.selected = idx;
            self.load_preview();
        }
        self.search_mode = false;
        self.search_query.clear();
        self.filtered_indices.clear();
        self.filtered_set.clear();
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    pub fn search_move_down(&mut self) {
        // Move to the next filtered match after current selection.
        if let Some(pos) = self
            .filtered_indices
            .iter()
            .position(|&i| i > self.selected)
        {
            self.selected = self.filtered_indices[pos];
            self.load_preview();
        }
    }

    pub fn search_move_up(&mut self) {
        // Move to the previous filtered match before current selection.
        if let Some(pos) = self
            .filtered_indices
            .iter()
            .rposition(|&i| i < self.selected)
        {
            self.selected = self.filtered_indices[pos];
            self.load_preview();
        }
    }

    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| fuzzy_match(&e.name.to_lowercase(), &query))
                .map(|(i, _)| i)
                .collect();
        }
        self.filtered_set = self.filtered_indices.iter().copied().collect();
        // Auto-select first match.
        if let Some(&first) = self.filtered_indices.first() {
            self.selected = first;
            self.load_preview();
        }
    }
}
