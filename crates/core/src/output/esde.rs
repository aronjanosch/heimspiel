//! ES-DE: per Entry a `.desktop` file (PC games) or a Placeholder (game files) in the system's ROM
//! folder, plus its `gamelist.xml` entry and cover.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::library::{Entry, FileLocation, Launch, Library, Metadata, SourceKind};

/// The folders of one ES-DE installation.
pub struct EsDe {
    home: PathBuf,
    roms: PathBuf,
    media: PathBuf,
    /// Where the `game-start` hook keeps fetched games; links into it replace Placeholders.
    game_cache: Option<PathBuf>,
}

impl EsDe {
    /// Reads the ROM and media folders from `<home>/settings/es_settings.xml`, falling back to
    /// ES-DE's defaults `~/ROMs` and `<home>/downloaded_media` (`user_home` stands in for `~`).
    pub fn locate(home: &Path, user_home: &Path) -> anyhow::Result<EsDe> {
        let settings =
            fs::read_to_string(home.join("settings/es_settings.xml")).unwrap_or_default();
        let roms = setting(&settings, "ROMDirectory")
            .map(|dir| expand_home(&dir, user_home))
            .unwrap_or_else(|| user_home.join("ROMs"));
        let media = setting(&settings, "MediaDirectory")
            .map(|dir| expand_home(&dir, user_home))
            .unwrap_or_else(|| home.join("downloaded_media"));
        Ok(EsDe {
            home: home.to_path_buf(),
            roms,
            media,
            game_cache: None,
        })
    }

    /// Tells the output where fetched games live, so it keeps links to them in place of
    /// their Placeholders.
    pub fn game_cache(mut self, dir: &Path) -> EsDe {
        self.game_cache = Some(dir.to_path_buf());
        self
    }

    /// Puts heimspiel's `game-start` script in place and turns on ES-DE's event scripts.
    /// `command` starts heimspiel. Returns whether anything changed.
    pub fn install_hook(&self, command: &[String]) -> anyhow::Result<bool> {
        use std::os::unix::fs::PermissionsExt;
        let script = self.home.join("scripts/game-start/heimspiel");
        let command: Vec<String> = command.iter().map(|a| shell_quote(a)).collect();
        let text = format!(
            "#!/bin/sh\n# Written by heimspiel sync: fetches RomM games before they start.\n\
             exec {} hook game-start \"$@\"\n",
            command.join(" ")
        );
        let mut changed = fs::read_to_string(&script).ok().as_deref() != Some(text.as_str());
        if changed {
            fs::create_dir_all(script.parent().unwrap())?;
            write_if_changed(&script, text.as_bytes())?;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
        }

        let path = self.home.join("settings/es_settings.xml");
        let settings = fs::read_to_string(&path).unwrap_or_default();
        let on = r#"<bool name="CustomEventScripts" value="true" />"#;
        if !settings.contains(on) {
            let mut lines: Vec<&str> = settings
                .lines()
                .filter(|l| !l.contains(r#"name="CustomEventScripts""#))
                .collect();
            if lines.is_empty() {
                lines.push(r#"<?xml version="1.0"?>"#);
            }
            lines.push(on);
            fs::create_dir_all(path.parent().unwrap())?;
            write_if_changed(&path, (lines.join("\n") + "\n").as_bytes())?;
            changed = true;
        }
        Ok(changed)
    }

    /// Brings every system in line with `library`. Files and gamelist entries that heimspiel did
    /// not write are left alone.
    pub fn write(&self, library: &Library) -> anyhow::Result<()> {
        let mut systems: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
        // Systems without Entries still need a pass, to remove what heimspiel wrote there before.
        if let Ok(dirs) = fs::read_dir(&self.roms) {
            for dir in dirs.filter_map(Result::ok) {
                if dir.path().is_dir() {
                    systems
                        .entry(dir.file_name().to_string_lossy().into_owned())
                        .or_default();
                }
            }
        }
        for entry in &library.entries {
            systems.entry(system(entry)).or_default().push(entry);
        }
        for (system, entries) in systems {
            self.write_system(&system, entries)?;
        }
        Ok(())
    }

    fn write_system(&self, system: &str, mut entries: Vec<&Entry>) -> anyhow::Result<()> {
        let dir = self.roms.join(system);
        if entries.is_empty() && !dir.exists() {
            return Ok(());
        }
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let own = own_files(&dir, self.game_cache.as_deref())?;
        let foreign = |file: &str| dir.join(file).exists() && !own.iter().any(|o| o == file);
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        let mut added: Vec<(String, &Entry)> = Vec::new();
        for entry in entries {
            let (stem, ext, content) = match &entry.launch {
                Launch::Command { .. } => (
                    file_stem(&entry.title),
                    ".desktop".to_owned(),
                    desktop_file(entry),
                ),
                Launch::File {
                    file_name,
                    location,
                    ..
                } => {
                    let name = file_stem(file_name);
                    let (stem, ext) = match name.rfind('.') {
                        Some(dot) if dot > 0 => (name[..dot].to_owned(), name[dot..].to_owned()),
                        _ => (name, String::new()),
                    };
                    (stem, ext, placeholder(location))
                }
            };
            // Most likely the same game, placed by the user or ES-DE's Game Importer.
            if foreign(&format!("{stem}{ext}")) {
                continue;
            }
            let file = (1..)
                .map(|n| match n {
                    1 => format!("{stem}{ext}"),
                    n => format!("{stem} ({n}){ext}"),
                })
                .find(|f| !foreign(f) && !added.iter().any(|(a, _)| a == f))
                .unwrap();
            // A fetched game stays linked; the hook made that link from this Placeholder.
            if !is_link_into(&dir.join(&file), self.game_cache.as_deref()) {
                write_if_changed(&dir.join(&file), content.as_bytes())?;
            }
            if let Some(cover) = &entry.cover {
                self.place_cover(system, &file, cover)?;
            }
            added.push((file, entry));
        }
        let mut removed = Vec::new();
        for file in own {
            if !added.iter().any(|(name, _)| *name == file) {
                fs::remove_file(dir.join(&file)).with_context(|| format!("removing {file}"))?;
                self.remove_covers(system, &file)?;
                removed.push(file);
            }
        }
        self.update_gamelist(system, &added, &removed)
    }

    /// Adds a `<game>` for every added file that has none yet and drops those of removed files;
    /// leaves every other `<game>` as ES-DE wrote it.
    fn update_gamelist(
        &self,
        system: &str,
        added: &[(String, &Entry)],
        removed: &[String],
    ) -> anyhow::Result<()> {
        let path = self
            .home
            .join("gamelists")
            .join(system)
            .join("gamelist.xml");
        if added.is_empty() && removed.is_empty() && !path.exists() {
            return Ok(());
        }
        let mut doc = Gamelist::read(&path)?;
        let list = doc.game_list();
        let before = list.children.len();
        list.children.retain(|n| {
            game_path_of(n).is_none_or(|p| !removed.iter().any(|file| p == format!("./{file}")))
        });
        let mut changed = list.children.len() != before;
        for (file, entry) in added {
            let game_path = format!("./{file}");
            if !list
                .children
                .iter()
                .any(|n| game_path_of(n).as_deref() == Some(game_path.as_str()))
            {
                list.children
                    .push(XMLNode::Element(game(&game_path, entry)));
                changed = true;
            }
        }
        if changed {
            fs::create_dir_all(path.parent().unwrap())?;
            write_if_changed(&path, doc.to_xml()?.as_bytes())?;
        }
        Ok(())
    }

    /// Copies `cover` to `<media>/<system>/covers/<file stem>.<ext>`, where ES-DE looks for it.
    fn place_cover(&self, system: &str, file: &str, cover: &Path) -> anyhow::Result<()> {
        let dir = self.media.join(system).join("covers");
        let ext = cover.extension().unwrap_or_default().to_string_lossy();
        let target = dir.join(format!("{}.{ext}", stem_of(file)));
        let bytes = fs::read(cover).with_context(|| format!("reading {}", cover.display()))?;
        fs::create_dir_all(&dir)?;
        write_if_changed(&target, &bytes)
    }

    fn remove_covers(&self, system: &str, file: &str) -> anyhow::Result<()> {
        let Ok(items) = fs::read_dir(self.media.join(system).join("covers")) else {
            return Ok(());
        };
        for item in items {
            let path = item?.path();
            if path.file_stem().is_some_and(|s| s == stem_of(file)) {
                fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            }
        }
        Ok(())
    }
}

/// A `gamelist.xml`. ES-DE puts `<alternativeEmulator>` next to `<gameList>`, so the file has several
/// top-level elements and is parsed inside a synthetic root.
struct Gamelist {
    top: Vec<Element>,
}

impl Gamelist {
    fn read(path: &Path) -> anyhow::Result<Gamelist> {
        let Ok(text) = fs::read_to_string(path) else {
            return Ok(Gamelist { top: Vec::new() });
        };
        let body = match text.find("?>") {
            Some(end) if text.trim_start().starts_with("<?xml") => &text[end + 2..],
            _ => &text,
        };
        let root = Element::parse(format!("<root>{body}</root>").as_bytes())
            .with_context(|| format!("parsing {}", path.display()))?;
        let top = root
            .children
            .into_iter()
            .filter_map(|n| n.as_element().cloned())
            .collect();
        Ok(Gamelist { top })
    }

    fn game_list(&mut self) -> &mut Element {
        if !self.top.iter().any(|e| e.name == "gameList") {
            self.top.push(Element::new("gameList"));
        }
        self.top.iter_mut().find(|e| e.name == "gameList").unwrap()
    }

    fn to_xml(&self) -> anyhow::Result<String> {
        let config = EmitterConfig::new()
            .perform_indent(true)
            .indent_string("\t")
            .write_document_declaration(false);
        let mut xml = b"<?xml version=\"1.0\"?>\n".to_vec();
        for element in &self.top {
            element.write_with_config(&mut xml, config.clone())?;
            xml.push(b'\n');
        }
        Ok(String::from_utf8(xml)?)
    }
}

/// Marks the `.desktop` files heimspiel owns; files without it belong to the user.
const MARKER: &str = "X-Heimspiel-Id=";

/// First line of every Placeholder heimspiel writes.
const PLACEHOLDER: &str = "heimspiel-placeholder ";

/// The ES-DE system an Entry goes into.
fn system(entry: &Entry) -> String {
    let platform = match (&entry.launch, entry.source) {
        (Launch::File { platform, .. }, _) => platform.as_str(),
        (_, SourceKind::Steam) => return "steam".into(),
        (_, SourceKind::Heroic) => return "epic".into(),
        (_, _) => return "desktop".into(),
    };
    // RomM platform slugs that ES-DE names differently; all others match.
    match platform {
        "dc" => "dreamcast",
        "g-and-w" => "gameandwatch",
        "lynx" => "atarilynx",
        "ngc" => "gc",
        "o2em" => "odyssey2",
        "prboom" => "doom",
        "sg1000" => "sg-1000",
        "sms" => "mastersystem",
        "snes-msu1" => "snes",
        "zxs" => "zxspectrum",
        other => other,
    }
    .to_owned()
}

fn desktop_file(entry: &Entry) -> String {
    let source = match entry.source {
        SourceKind::Steam => "steam",
        SourceKind::Heroic => "heroic",
        SourceKind::Manual => "manual",
        SourceKind::RomM => "romm",
    };
    let Launch::Command { exec, working_dir } = &entry.launch else {
        unreachable!("only Command Entries become .desktop files");
    };
    let mut text = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={exec}\n{MARKER}{source}:{}\n",
        entry.title, entry.id
    );
    if let Some(dir) = working_dir {
        text += &format!("Path={}\n", dir.display());
    }
    text
}

fn placeholder(location: &FileLocation) -> String {
    let FileLocation::RomM { rom_id } = location;
    format!("{PLACEHOLDER}romm:{rom_id}\n")
}

/// Names of the files in `dir` that heimspiel wrote: marked `.desktop` files and Placeholders.
/// Only small files are opened, so real game files are never read.
fn own_files(dir: &Path, game_cache: Option<&Path>) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        let meta = item.metadata()?;
        if meta.is_symlink() && is_link_into(&item.path(), game_cache) {
            files.push(item.file_name().to_string_lossy().into_owned());
            continue;
        }
        if !meta.is_file() || meta.len() > 64 * 1024 {
            continue;
        }
        let path = item.path();
        let mut head = String::new();
        let _ = fs::File::open(&path)?
            .take(64 * 1024)
            .read_to_string(&mut head);
        let is_own = if path.extension().is_some_and(|e| e == "desktop") {
            head.lines().any(|l| l.starts_with(MARKER))
        } else {
            head.starts_with(PLACEHOLDER)
        };
        if is_own && let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            files.push(name.to_owned());
        }
    }
    Ok(files)
}

/// The file name without its last extension, as ES-DE names media files.
fn stem_of(file: &str) -> &str {
    match file.rfind('.') {
        Some(dot) if dot > 0 => &file[..dot],
        _ => file,
    }
}

/// A safe file name: no path separators, no leading dots.
fn file_stem(title: &str) -> String {
    title
        .replace(['/', '\\'], "-")
        .trim_start_matches('.')
        .to_owned()
}

fn game_path_of(node: &XMLNode) -> Option<String> {
    let game = node.as_element().filter(|e| e.name == "game")?;
    Some(game.get_child("path")?.get_text()?.into_owned())
}

/// A new `<game>` with the Entry's title and Metadata, in ES-DE's field formats.
fn game(path: &str, entry: &Entry) -> Element {
    let Metadata {
        description,
        release_date,
        developer,
        publisher,
        genre,
        players,
        rating,
    } = &entry.metadata;
    let mut game = Element::new("game");
    let mut add = |name: &str, value: Option<String>| {
        if let Some(value) = value {
            game.children
                .push(XMLNode::Element(text_element(name, &value)));
        }
    };
    add("path", Some(path.to_owned()));
    add("name", Some(entry.title.clone()));
    add("desc", description.clone());
    // ES-DE rates in half stars: tenths between 0 and 1.
    add("rating", rating.map(|r| format!("{r:.1}")));
    add(
        "releasedate",
        release_date
            .as_ref()
            .map(|d| format!("{}T000000", d.replace('-', ""))),
    );
    add("developer", developer.clone());
    add("publisher", publisher.clone());
    add("genre", genre.clone());
    add("players", players.clone());
    game
}

fn text_element(name: &str, text: &str) -> Element {
    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(text.to_owned()));
    element
}

/// Whether `path` is a link into `dir`, dangling or not.
fn is_link_into(path: &Path, dir: Option<&Path>) -> bool {
    dir.is_some_and(|dir| fs::read_link(path).is_ok_and(|target| target.starts_with(dir)))
}

/// Quotes an argument for `/bin/sh`.
fn shell_quote(arg: &str) -> String {
    if !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+=:".contains(c))
    {
        return arg.to_owned();
    }
    format!("'{}'", arg.replace('\'', r"'\''"))
}

/// Writes atomically, and only when the content differs, so unchanged files keep their mtime.
fn write_if_changed(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    if fs::read(path).is_ok_and(|old| old == content) {
        return Ok(());
    }
    let tmp = path.with_extension("heimspiel-tmp");
    fs::write(&tmp, content).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// The value of `<string name="NAME" value="…" />` in `es_settings.xml`, which has no root element.
fn setting(settings: &str, name: &str) -> Option<String> {
    let start = settings.find(&format!("name=\"{name}\""))?;
    let rest = &settings[start..];
    let value = rest[rest.find("value=\"")? + 7..].split('"').next()?;
    let value = value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    (!value.is_empty()).then_some(value)
}

fn expand_home(dir: &str, user_home: &Path) -> PathBuf {
    match dir.strip_prefix("~/") {
        Some(rest) => user_home.join(rest),
        None => PathBuf::from(dir),
    }
}
