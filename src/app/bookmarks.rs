use super::App;

impl App {
    /// Bookmark the current directory.
    pub fn add_bookmark(&mut self) {
        match crate::bookmarks::add(&self.cwd) {
            Ok(()) => self.status_message = Some(format!("Bookmarked {}", self.cwd.display())),
            Err(e) => self.status_message = Some(format!("Bookmark failed: {e}")),
        }
    }

    /// Open the bookmark picker overlay.
    pub fn open_bookmarks(&mut self) {
        self.bookmarks = crate::bookmarks::load();
        self.bookmark_query.clear();
        self.bookmark_filtered = (0..self.bookmarks.len()).collect();
        self.bookmark_selected = 0;
        self.bookmark_mode = true;
    }

    /// Close the picker without navigating.
    pub fn close_bookmarks(&mut self) {
        self.bookmark_mode = false;
        self.bookmark_query.clear();
    }

    /// Navigate to the currently focused bookmark.
    pub fn confirm_bookmark(&mut self) {
        let Some(&real_idx) = self.bookmark_filtered.get(self.bookmark_selected) else {
            return;
        };
        let Some(dest) = self.bookmarks.get(real_idx).cloned() else {
            return;
        };
        self.close_bookmarks();
        if !dest.is_dir() {
            self.status_message = Some(format!("\"{}\" no longer exists", dest.display()));
            return;
        }
        self.filter_input.clear();
        self.filter_mode = false;
        self.push_history(dest.clone());
        self.cwd = dest;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
    }

    /// Remove the focused bookmark from disk immediately.
    pub fn remove_bookmark(&mut self) {
        let Some(&real_idx) = self.bookmark_filtered.get(self.bookmark_selected) else {
            return;
        };
        let _ = crate::bookmarks::remove(real_idx);
        self.bookmarks = crate::bookmarks::load();
        self.update_bookmark_filter();
        if !self.bookmark_filtered.is_empty() {
            self.bookmark_selected = self
                .bookmark_selected
                .min(self.bookmark_filtered.len().saturating_sub(1));
        }
    }

    /// Append a character to the bookmark filter and re-filter.
    pub fn bookmark_push_char(&mut self, c: char) {
        self.bookmark_query.push(c);
        self.update_bookmark_filter();
        self.bookmark_selected = 0;
    }

    /// Remove the last character from the bookmark filter and re-filter.
    pub fn bookmark_pop_char(&mut self) {
        self.bookmark_query.pop();
        self.update_bookmark_filter();
        self.bookmark_selected = 0;
    }

    /// Move selection up in the bookmark picker.
    pub fn bookmark_move_up(&mut self) {
        self.bookmark_selected = self.bookmark_selected.saturating_sub(1);
    }

    /// Move selection down in the bookmark picker.
    pub fn bookmark_move_down(&mut self) {
        if !self.bookmark_filtered.is_empty()
            && self.bookmark_selected + 1 < self.bookmark_filtered.len()
        {
            self.bookmark_selected += 1;
        }
    }

    /// Recompute `bookmark_filtered` from the current query.
    fn update_bookmark_filter(&mut self) {
        if self.bookmark_query.is_empty() {
            self.bookmark_filtered = (0..self.bookmarks.len()).collect();
            return;
        }
        let q = self.bookmark_query.to_lowercase();
        self.bookmark_filtered = self
            .bookmarks
            .iter()
            .enumerate()
            .filter(|(_, p)| p.to_string_lossy().to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
    }
}
