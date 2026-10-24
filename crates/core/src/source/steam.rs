//! Steam: installed games from the app manifests of every Steam library folder.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::library::{Entry, SourceKind};

/// `StateFlags` bit Steam sets once an app is fully installed.
const FULLY_INSTALLED: u32 = 4;

/// Steam installs these alongside games; they are not playable.
const TOOL_PREFIXES: &[&str] = &[
    "Proton",
    "Steam Linux Runtime",
    "Steamworks Common Redistributables",
];

/// Reads the installed games of the Steam installation at `root` (the folder holding `steamapps/`).
/// `command` is the program that receives the `steam://rungameid/<id>` URL.
pub fn entries(root: &Path, command: &str) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for library in library_folders(root)? {
        let Ok(dir) = fs::read_dir(library.join("steamapps")) else {
            continue;
        };
        for file in dir {
            let path = file?.path();
            let is_manifest = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"));
            if !is_manifest {
                continue;
            }
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if let Some(entry) = entry_from_manifest(&text, root, command) {
                entries.push(entry);
            }
        }
    }
    Ok(entries)
}

fn entry_from_manifest(text: &str, root: &Path, command: &str) -> Option<Entry> {
    let appid = value(text, "appid")?;
    let name = value(text, "name")?;
    let flags: u32 = value(text, "StateFlags")?.parse().ok()?;
    if flags & FULLY_INSTALLED == 0 || TOOL_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return None;
    }
    Some(Entry {
        source: SourceKind::Steam,
        exec: format!("{command} steam://rungameid/{appid}"),
        cover: cover(root, appid),
        id: appid.to_owned(),
        title: name.to_owned(),
        working_dir: None,
    })
}

/// The library folders listed in `libraryfolders.vdf`; `root` alone if the file is missing.
fn library_folders(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let vdf = root.join("steamapps/libraryfolders.vdf");
    let Ok(text) = fs::read_to_string(&vdf) else {
        return Ok(vec![root.to_path_buf()]);
    };
    let mut folders: Vec<PathBuf> = Vec::new();
    for line in text.lines() {
        if let Some(("path", path)) = key_value(line) {
            let path = PathBuf::from(path);
            let known = folders.iter().any(|f| same_dir(f, &path));
            if !known {
                folders.push(path);
            }
        }
    }
    if !folders.iter().any(|f| same_dir(f, root)) {
        folders.push(root.to_path_buf());
    }
    Ok(folders)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// Steam has used both `<id>/library_600x900.jpg` and `<id>_library_600x900.jpg` in its cache.
fn cover(root: &Path, appid: &str) -> Option<PathBuf> {
    let cache = root.join("appcache/librarycache");
    [
        cache.join(appid).join("library_600x900.jpg"),
        cache.join(format!("{appid}_library_600x900.jpg")),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// The value of the first top-level `"key" "value"` line with this key.
fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .filter_map(key_value)
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
}

/// Splits a KeyValues line `"key"  "value"` into its two strings.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split('"');
    let (_, key, _, value) = (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    Some((key, value))
}
