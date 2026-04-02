use super::App;

impl App {
    /// Enter recursive filename find mode.
    pub fn start_find(&mut self) {
        self.overlay.find_mode = true;
        self.overlay.find_query.clear();
        self.overlay.find_results.clear();
        self.overlay.find_selected = 0;
        self.overlay.find_error = None;
        self.overlay.find_truncated = false;
    }

    /// Exit find mode without side effects.
    pub fn cancel_find(&mut self) {
        self.overlay.find_mode = false;
        self.overlay.find_query.clear();
        self.overlay.find_results.clear();
        self.overlay.find_selected = 0;
        self.overlay.find_error = None;
        self.overlay.find_truncated = false;
    }

    /// Append a character to the find query and re-run the search.
    pub fn find_push_char(&mut self, c: char) {
        self.overlay.find_query.push(c);
        self.overlay.find_selected = 0;
        self.exec_find();
    }

    /// Remove the last character from the find query and re-run the search.
    pub fn find_pop_char(&mut self) {
        self.overlay.find_query.pop();
        self.overlay.find_selected = 0;
        self.exec_find();
    }

    /// Move the find selection down by one result.
    pub fn find_move_down(&mut self) {
        if !self.overlay.find_results.is_empty()
            && self.overlay.find_selected + 1 < self.overlay.find_results.len()
        {
            self.overlay.find_selected += 1;
        }
    }

    /// Move the find selection up by one result.
    pub fn find_move_up(&mut self) {
        self.overlay.find_selected = self.overlay.find_selected.saturating_sub(1);
    }

    /// Execute the find query against the current working directory.
    fn exec_find(&mut self) {
        match crate::find::run_find(&self.overlay.find_query, &self.nav.cwd) {
            Ok(results) => {
                self.overlay.find_truncated = results.len() >= crate::find::MAX_FIND_RESULTS;
                self.overlay.find_results = results;
                self.overlay.find_error = None;
            }
            Err(e) => {
                self.overlay.find_results.clear();
                self.overlay.find_error = Some(e);
                self.overlay.find_truncated = false;
            }
        }
    }

    /// Navigate to the currently selected find result: change `cwd` to the
    /// file's parent directory, select the file, exit find mode, and push a
    /// history entry.
    pub fn jump_to_find_result(&mut self) {
        let Some(result) = self
            .overlay
            .find_results
            .get(self.overlay.find_selected)
            .cloned()
        else {
            return;
        };

        let file_path = result.absolute;
        let Some(parent) = file_path.parent() else {
            return;
        };

        self.nav.filter_input.clear();
        self.nav.filter_mode = false;

        if parent != self.nav.cwd {
            let new_dir = parent.to_path_buf();
            self.push_history(new_dir.clone());
            self.nav.cwd = new_dir;
            self.nav.selected = 0;
            self.nav.current_scroll = 0;
            self.load_dir();
        }

        // Select the file in the entry list.
        if let Some(name) = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
        {
            if let Some(idx) = self.nav.entries.iter().position(|e| e.name == name) {
                self.nav.selected = idx;
                self.load_preview();
            }
        }

        self.cancel_find();
    }
}
