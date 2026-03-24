use std::path::PathBuf;
use std::time::Instant;

/// One entry in the session frecency table.
#[derive(Debug, Clone)]
pub struct FrecencyEntry {
    pub path: PathBuf,
    pub visits: u32,
    pub last_visit: Instant,
}

impl FrecencyEntry {
    /// Frecency score = visits × recency_weight.
    ///
    /// Recency weight is a step function that strongly favours recent visits:
    /// - Within 1 hour  → 4.0
    /// - Within 24 hours → 2.0
    /// - Within 1 week   → 1.0
    /// - Older           → 0.5
    pub fn score(&self) -> f64 {
        let secs = self.last_visit.elapsed().as_secs();
        let weight = if secs < 3_600 {
            4.0
        } else if secs < 86_400 {
            2.0
        } else if secs < 604_800 {
            1.0
        } else {
            0.5
        };
        self.visits as f64 * weight
    }
}
