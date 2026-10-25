//! Telling the user what a long step does while ES-DE waits for heimspiel, such as a download or
//! the installation of a Port. ES-DE shows only a black screen meanwhile.

use std::path::Path;

pub trait Progress {
    /// A long step starts: `action`, such as "Downloading", on `game`, with its cover if known.
    fn start(&self, game: &str, cover: Option<&Path>, action: &str);
    /// How much of the step is done.
    fn advance(&self, done_bytes: u64, total_bytes: u64);
    /// The steps are over and the game starts next.
    fn finish(&self);
    /// Shows that the game cannot start, and why, until the user has seen it.
    fn fail(&self, message: &str);
}

/// For callers that show nothing.
impl Progress for () {
    fn start(&self, _: &str, _: Option<&Path>, _: &str) {}
    fn advance(&self, _: u64, _: u64) {}
    fn finish(&self) {}
    fn fail(&self, _: &str) {}
}
