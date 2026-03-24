use super::App;
use crate::search;

impl App {
    /// Enter content search mode.
    pub fn start_content_search(&mut self) {
        self.content_search_mode = true;
        self.content_search_query.clear();
        self.content_search_results.clear();
        self.content_search_selected = 0;
        self.content_search_error = None;
        self.content_search_truncated = false;
    }

    /// Exit content search mode without side effects.
    pub fn cancel_content_search(&mut self) {
        self.content_search_mode = false;
        self.content_search_query.clear();
        self.content_search_results.clear();
        self.content_search_selected = 0;
        self.content_search_error = None;
        self.content_search_truncated = false;
    }

    pub fn content_search_push_char(&mut self, c: char) {
        self.content_search_query.push(c);
    }

    pub fn content_search_pop_char(&mut self) {
        self.content_search_query.pop();
    }

    /// Run rg with the current query and populate results.
    pub fn run_content_search(&mut self) {
        if self.content_search_query.is_empty() {
            return;
        }
        match search::run_rg(&self.content_search_query, &self.cwd) {
            Ok(groups) => {
                let total: usize = groups.iter().map(|g| g.matches.len()).sum();
                self.content_search_truncated = total >= search::MAX_RESULTS;
                self.content_search_results = groups;
                self.content_search_selected = 0;
                self.content_search_error = None;
            }
            Err(e) => {
                self.content_search_results.clear();
                self.content_search_error = Some(e);
            }
        }
    }

    /// Move selection down by one match entry (crosses file boundaries).
    pub fn content_search_move_down(&mut self) {
        let total: usize = self
            .content_search_results
            .iter()
            .map(|g| g.matches.len())
            .sum();
        if total > 0 && self.content_search_selected + 1 < total {
            self.content_search_selected += 1;
        }
    }

    /// Move selection up by one match entry.
    pub fn content_search_move_up(&mut self) {
        self.content_search_selected = self.content_search_selected.saturating_sub(1);
    }

    /// Navigate to the currently selected search result: update cwd if needed,
    /// select the file in the entry list, and scroll the preview to the match line.
    pub fn jump_to_content_result(&mut self) {
        // Resolve flat index → (group, match).
        let mut flat = self.content_search_selected;
        let mut target_file: Option<std::path::PathBuf> = None;
        let mut target_line: u64 = 0;
        for group in &self.content_search_results {
            if flat < group.matches.len() {
                target_file = Some(self.cwd.join(&group.file));
                target_line = group.matches[flat].line_number;
                break;
            }
            flat -= group.matches.len();
        }
        let Some(file_path) = target_file else {
            return;
        };
        // Navigate to the file's parent directory if different from cwd.
        if let Some(parent) = file_path.parent() {
            if parent != self.cwd {
                let new_dir = parent.to_path_buf();
                self.push_history(new_dir.clone());
                self.cwd = new_dir;
                self.selected = 0;
                self.current_scroll = 0;
                self.load_dir();
            }
        }
        // Select the file in the entry list.
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        if let Some(name) = file_name {
            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected = idx;
                self.load_preview();
                // Scroll preview to the matching line (1-based → 0-based offset).
                self.preview_scroll = (target_line as usize).saturating_sub(1);
            }
        }
    }
}
