use super::{App, SortMode, SortOrder};

impl App {
    /// Cycle through Name → Size → Modified → Extension → Name.
    ///
    /// Size and Modified default to descending (most useful first); the others
    /// default to ascending.
    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.sort_order = match self.sort_mode {
            SortMode::Size | SortMode::Modified => SortOrder::Descending,
            _ => SortOrder::Ascending,
        };
        self.apply_sort();
        let arrow = if self.sort_order == SortOrder::Descending {
            "↓"
        } else {
            "↑"
        };
        self.status_message = Some(format!("Sort: {} {}", self.sort_mode.label(), arrow));
    }

    /// Toggle the sort direction between ascending and descending.
    pub fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
        self.apply_sort();
    }

    fn apply_sort(&mut self) {
        // Capture selected file's name so the cursor follows the file after re-sort.
        let selected_name = self.entries.get(self.selected).map(|e| e.name.clone());
        Self::sort_entries(&mut self.entries, self.sort_mode, self.sort_order);
        // parent_entries is only used as a directory indicator in the left pane;
        // its display order has no user-visible effect, so we skip sorting it.
        if let Some(name) = selected_name {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
            }
        }
        self.load_preview();
    }
}
