use super::digest::SessionDigest;
use super::lenses::{Thresholds, select_sessions_by_lenses};

/// High-level API for analyzing and selecting interesting sessions from a corpus
pub fn analyze_and_select_sessions(
    digests: Vec<SessionDigest>,
    limit: usize,
) -> Vec<SessionDigest> {
    if digests.is_empty() {
        return Vec::new();
    }

    let thresholds = Thresholds::compute(&digests);
    select_sessions_by_lenses(&digests, &thresholds, limit)
}
