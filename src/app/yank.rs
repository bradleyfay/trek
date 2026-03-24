use super::App;
use std::io::Write;

impl App {
    pub fn yank_relative_path(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let rel = entry.path.strip_prefix(&self.cwd).unwrap_or(&entry.path);
            let path_str = format!("./{}", rel.display());
            self.osc52_copy(&path_str);
            self.status_message = Some(format!("Yanked: {}", path_str));
        }
    }

    pub fn yank_absolute_path(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            let path_str = entry.path.to_string_lossy().into_owned();
            self.osc52_copy(&path_str);
            self.status_message = Some(format!("Yanked: {}", path_str));
        }
    }

    /// Write an OSC 52 sequence to set the system clipboard.
    fn osc52_copy(&self, text: &str) {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        // OSC 52 ; c ; <base64> ST
        let seq = format!("\x1b]52;c;{}\x07", encoded);
        let _ = std::io::stdout().write_all(seq.as_bytes());
        let _ = std::io::stdout().flush();
    }
}
