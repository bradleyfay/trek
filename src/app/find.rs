use super::App;

impl App {
    /// Enter recursive filename find mode.
    pub fn start_find(&mut self) {
        self.find_mode = true;
        self.find_query.clear();
        self.find_results.clear();
        self.find_selected = 0;
        self.find_error = None;
        self.find_truncated = false;
    }

    /// Exit find mode without side effects.
    pub fn cancel_find(&mut self) {
        self.find_mode = false;
        self.find_query.clear();
        self.find_results.clear();
        self.find_selected = 0;
        self.find_error = None;
        self.find_truncated = false;
    }

    /// Append a character to the find query and re-run the search.
    pub fn find_push_char(&mut self, c: char) {
        self.find_query.push(c);
        self.find_selected = 0;
        self.exec_find();
    }

    /// Remove the last character from the find query and re-run the search.
    pub fn find_pop_char(&mut self) {
        self.find_query.pop();
        self.find_selected = 0;
        self.exec_find();
    }

    /// Move the find selection down by one result.
    pub fn find_move_down(&mut self) {
        if !self.find_results.is_empty() && self.find_selected + 1 < self.find_results.len() {
            self.find_selected += 1;
        }
    }

    /// Move the find selection up by one result.
    pub fn find_move_up(&mut self) {
        self.find_selected = self.find_selected.saturating_sub(1);
    }

    /// Execute the find query against the current working directory.
    fn exec_find(&mut self) {
        match crate::find::run_find(&self.find_query, &self.cwd) {
            Ok(results) => {
                self.find_truncated = results.len() >= crate::find::MAX_FIND_RESULTS;
                self.find_results = results;
                self.find_error = None;
            }
            Err(e) => {
                self.find_results.clear();
                self.find_error = Some(e);
                self.find_truncated = false;
            }
        }
    }

    /// Navigate to the currently selected find result: change `cwd` to the
    /// file's parent directory, select the file, exit find mode, and push a
    /// history entry.
    pub fn jump_to_find_result(&mut self) {
        let Some(result) = self.find_results.get(self.find_selected).cloned() else {
            return;
        };

        let file_path = result.absolute;
        let Some(parent) = file_path.parent() else {
            return;
        };

        self.filter_input.clear();
        self.filter_mode = false;

        if parent != self.cwd {
            let new_dir = parent.to_path_buf();
            self.push_history(new_dir.clone());
            self.cwd = new_dir;
            self.selected = 0;
            self.current_scroll = 0;
            self.load_dir();
        }

        // Select the file in the entry list.
        if let Some(name) = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
        {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
                self.load_preview();
            }
        }

        self.cancel_find();
    }
}
