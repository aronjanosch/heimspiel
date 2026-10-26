//! What a frontend starts of a game made of several files, wherever the files come from.

/// What a frontend starts of a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// This file, relative to the game's folder.
    File(String),
    /// These discs, in this order, through a playlist (`.m3u`) that heimspiel writes.
    Playlist(Vec<String>),
    /// The game's folder itself.
    Folder,
}

/// Disc descriptions that name the tracks next to them, in order of preference.
const DISC_INDEXES: &[&str] = &["m3u", "cue", "gdi", "ccd", "mds", "toc"];

/// Disc images complete in themselves; several of them make a multi-disc game.
const DISC_IMAGES: &[&str] = &["chd", "pbp", "cso", "zso", "cdi", "iso", "rvz", "wbfs"];

/// Files that accompany the file a frontend starts: tracks, MSU-1 audio, notes, checksums.
const COMPANIONS: &[&str] = &[
    "bin", "raw", "img", "sub", "wav", "mp3", "ogg", "pcm", "msu", "bml", "xml", "txt", "nfo",
    "md5", "sfv", "png", "jpg", "pdf",
];

/// What is started of the game in `folder`, holding the files `top` directly (not those in
/// subfolders), or `None` when the files do not tell.
pub fn of(folder: &str, top: &[&str]) -> Option<Start> {
    // A folder such as `Day of the Tentacle.svm` is opened as a whole. `Game (v1.1)` has no
    // extension, though its last dot suggests `1)`.
    let ext = extension(folder);
    if !ext.is_empty() && ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(Start::Folder);
    }
    let mut top = top.to_vec();
    top.sort_unstable();
    let with = |exts: &[&str]| -> Vec<&str> {
        top.iter()
            .copied()
            .filter(|name| exts.contains(&extension(name).as_str()))
            .collect()
    };
    for index in DISC_INDEXES {
        if let Some(file) = with(&[index]).first() {
            return Some(Start::File((*file).to_owned()));
        }
    }
    let images = with(DISC_IMAGES);
    match images.len() {
        0 => {}
        1 => return Some(Start::File(images[0].to_owned())),
        _ => {
            return Some(Start::Playlist(
                images.into_iter().map(String::from).collect(),
            ));
        }
    }
    let mut rest = top
        .iter()
        .filter(|name| !COMPANIONS.contains(&extension(name).as_str()));
    match (rest.next(), rest.next()) {
        (Some(file), None) => Some(Start::File((*file).to_owned())),
        _ => None,
    }
}

impl Start {
    /// The name a frontend shows for the game in `folder`, ending in what it starts: `Metal Slug
    /// 6.gdi`, `Final Fantasy VII.m3u`, or the folder's own name.
    pub fn name(&self, folder: &str) -> Option<String> {
        let extension = match self {
            Start::Folder => return Some(folder.to_owned()),
            Start::File(file) => file.rsplit_once('.')?.1,
            Start::Playlist(_) => "m3u",
        };
        Some(format!("{folder}.{extension}"))
    }
}

/// The lowercase extension of a file name; empty without one.
fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}
