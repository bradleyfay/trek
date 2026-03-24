use super::App;

impl App {
    /// Toggle gitignore-aware filtering on/off.
    ///
    /// When enabled, entries whose names are returned by `git ls-files --ignored`
    /// are removed from the current listing. When outside a git repository the
    /// toggle is a no-op and an informative status message is shown.
    pub fn toggle_gitignored(&mut self) {
        if self.git_status.is_none() {
            self.status_message = Some("Not in a git repository".to_string());
            return;
        }
        self.hide_gitignored = !self.hide_gitignored;
        self.selected = 0;
        self.current_scroll = 0;
        self.load_dir();
        if self.hide_gitignored {
            self.status_message = Some("Gitignore filter: on".to_string());
        } else {
            self.status_message = Some("Gitignore filter: off".to_string());
        }
    }
}
