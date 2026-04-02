use super::App;

impl App {
    /// Bookmark the current directory.
    pub fn add_bookmark(&mut self) {
        match crate::bookmarks::add(&self.nav.cwd) {
            Ok(()) => self.status_message = Some(format!("Bookmarked {}", self.nav.cwd.display())),
            Err(e) => self.status_message = Some(format!("Bookmark failed: {e}")),
        }
    }

    /// Open the bookmark picker overlay.
    pub fn open_bookmarks(&mut self) {
        self.overlay.bookmarks = crate::bookmarks::load();
        self.overlay.bookmark_query.clear();
        self.overlay.bookmark_filtered = (0..self.overlay.bookmarks.len()).collect();
        self.overlay.bookmark_selected = 0;
        self.overlay.bookmark_mode = true;
    }

    /// Close the picker without navigating.
    pub fn close_bookmarks(&mut self) {
        self.overlay.bookmark_mode = false;
        self.overlay.bookmark_query.clear();
    }

    /// Navigate to the currently focused bookmark.
    pub fn confirm_bookmark(&mut self) {
        let Some(&real_idx) = self
            .overlay
            .bookmark_filtered
            .get(self.overlay.bookmark_selected)
        else {
            return;
        };
        let Some(dest) = self.overlay.bookmarks.get(real_idx).cloned() else {
            return;
        };
        self.close_bookmarks();
        if !dest.is_dir() {
            self.status_message = Some(format!("\"{}\" no longer exists", dest.display()));
            return;
        }
        self.nav.filter_input.clear();
        self.nav.filter_mode = false;
        self.push_history(dest.clone());
        self.nav.cwd = dest;
        self.nav.selected = 0;
        self.nav.current_scroll = 0;
        self.load_dir();
    }

    /// Remove the focused bookmark from disk immediately.
    pub fn remove_bookmark(&mut self) {
        let Some(&real_idx) = self
            .overlay
            .bookmark_filtered
            .get(self.overlay.bookmark_selected)
        else {
            return;
        };
        let _ = crate::bookmarks::remove(real_idx);
        self.overlay.bookmarks = crate::bookmarks::load();
        self.update_bookmark_filter();
        if !self.overlay.bookmark_filtered.is_empty() {
            self.overlay.bookmark_selected = self
                .overlay
                .bookmark_selected
                .min(self.overlay.bookmark_filtered.len().saturating_sub(1));
        }
    }

    /// Append a character to the bookmark filter and re-filter.
    pub fn bookmark_push_char(&mut self, c: char) {
        self.overlay.bookmark_query.push(c);
        self.update_bookmark_filter();
        self.overlay.bookmark_selected = 0;
    }

    /// Remove the last character from the bookmark filter and re-filter.
    pub fn bookmark_pop_char(&mut self) {
        self.overlay.bookmark_query.pop();
        self.update_bookmark_filter();
        self.overlay.bookmark_selected = 0;
    }

    /// Move selection up in the bookmark picker.
    pub fn bookmark_move_up(&mut self) {
        self.overlay.bookmark_selected = self.overlay.bookmark_selected.saturating_sub(1);
    }

    /// Move selection down in the bookmark picker.
    pub fn bookmark_move_down(&mut self) {
        if !self.overlay.bookmark_filtered.is_empty()
            && self.overlay.bookmark_selected + 1 < self.overlay.bookmark_filtered.len()
        {
            self.overlay.bookmark_selected += 1;
        }
    }

    /// Recompute `bookmark_filtered` from the current query.
    fn update_bookmark_filter(&mut self) {
        if self.overlay.bookmark_query.is_empty() {
            self.overlay.bookmark_filtered = (0..self.overlay.bookmarks.len()).collect();
            return;
        }
        let q = self.overlay.bookmark_query.to_lowercase();
        self.overlay.bookmark_filtered = self
            .overlay
            .bookmarks
            .iter()
            .enumerate()
            .filter(|(_, p)| p.to_string_lossy().to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
    }
}
