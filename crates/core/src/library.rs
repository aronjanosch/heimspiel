//! The Library: every Entry heimspiel knows about, across all Sources.

use std::path::PathBuf;

/// All Entries heimspiel knows about.
#[derive(Debug, Default)]
pub struct Library {
    pub entries: Vec<Entry>,
}

/// One playable game, as delivered by exactly one Source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub source: SourceKind,
    /// Identifies the Entry within its Source (Steam app ID, Heroic `runner:appName`, …).
    pub id: String,
    pub title: String,
    /// Command line that starts the game, in `.desktop` `Exec` syntax.
    pub exec: String,
    /// Working directory for `exec`, if the game needs one.
    pub working_dir: Option<PathBuf>,
    /// Local cover image, if the Source has one.
    pub cover: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Steam,
    Heroic,
    Manual,
}
