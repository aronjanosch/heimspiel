//! ES-DE: one `.desktop` file per Entry in the system's ROM folder, plus `gamelist.xml` and covers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::library::{Entry, Library, SourceKind};

/// The folders of one ES-DE installation.
pub struct EsDe {
    home: PathBuf,
    roms: PathBuf,
    media: PathBuf,
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
        })
    }

    /// Brings every system heimspiel fills in line with `library`. Files and gamelist entries that
    /// heimspiel did not write are left alone.
    pub fn write(&self, library: &Library) -> anyhow::Result<()> {
        let mut systems: BTreeMap<&str, Vec<&Entry>> =
            SYSTEMS.iter().map(|s| (*s, Vec::new())).collect();
        for entry in &library.entries {
            systems.entry(system(entry.source)).or_default().push(entry);
        }
        for (system, entries) in systems {
            let dir = self.roms.join(system);
            if entries.is_empty() && !dir.exists() {
                continue;
            }
            fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            let own = own_files(&dir)?;
            let foreign = |file: &str| dir.join(file).exists() && !own.iter().any(|o| o == file);
            let mut entries = entries;
            entries.sort_by(|a, b| a.id.cmp(&b.id));
            let mut added: Vec<(String, &str)> = Vec::new();
            for entry in entries {
                let stem = file_stem(&entry.title);
                // Most likely the same game, placed by the user or ES-DE's Game Importer.
                if foreign(&format!("{stem}.desktop")) {
                    continue;
                }
                let file = (1..)
                    .map(|n| match n {
                        1 => format!("{stem}.desktop"),
                        n => format!("{stem} ({n}).desktop"),
                    })
                    .find(|f| !foreign(f) && !added.iter().any(|(a, _)| a == f))
                    .unwrap();
                write_if_changed(&dir.join(&file), desktop_file(entry).as_bytes())?;
                if let Some(cover) = &entry.cover {
                    self.place_cover(system, &file, cover)?;
                }
                added.push((file, entry.title.as_str()));
            }
            let mut removed = Vec::new();
            for file in own {
                if !added.iter().any(|(name, _)| *name == file) {
                    fs::remove_file(dir.join(&file)).with_context(|| format!("removing {file}"))?;
                    self.remove_covers(system, &file)?;
                    removed.push(file);
                }
            }
            self.update_gamelist(system, &added, &removed)?;
        }
        Ok(())
    }

    /// Adds a `<game>` for every added file that has none yet and drops those of removed files;
    /// leaves every other `<game>` as ES-DE wrote it.
    fn update_gamelist(
        &self,
        system: &str,
        added: &[(String, &str)],
        removed: &[String],
    ) -> anyhow::Result<()> {
        let path = self
            .home
            .join("gamelists")
            .join(system)
            .join("gamelist.xml");
        let mut doc = Gamelist::read(&path)?;
        let list = doc.game_list();
        let before = list.children.len();
        list.children.retain(|n| {
            game_path_of(n).is_none_or(|p| !removed.iter().any(|file| p == format!("./{file}")))
        });
        let mut changed = list.children.len() != before;
        for (file, title) in added {
            let game_path = format!("./{file}");
            if !list
                .children
                .iter()
                .any(|n| game_path_of(n).as_deref() == Some(game_path.as_str()))
            {
                let mut game = Element::new("game");
                game.children
                    .push(XMLNode::Element(text_element("path", &game_path)));
                game.children
                    .push(XMLNode::Element(text_element("name", title)));
                list.children.push(XMLNode::Element(game));
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

/// Every ES-DE system heimspiel writes into.
const SYSTEMS: [&str; 3] = ["steam", "epic", "desktop"];

/// The ES-DE system each Source's games go into.
fn system(source: SourceKind) -> &'static str {
    match source {
        SourceKind::Steam => "steam",
        SourceKind::Heroic => "epic",
        SourceKind::Manual => "desktop",
    }
}

fn desktop_file(entry: &Entry) -> String {
    let source = match entry.source {
        SourceKind::Steam => "steam",
        SourceKind::Heroic => "heroic",
        SourceKind::Manual => "manual",
    };
    let mut text = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={}\n{MARKER}{source}:{}\n",
        entry.title, entry.exec, entry.id
    );
    if let Some(dir) = &entry.working_dir {
        text += &format!("Path={}\n", dir.display());
    }
    text
}

/// Names of the `.desktop` files in `dir` that carry heimspiel's marker.
fn own_files(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for item in fs::read_dir(dir)? {
        let path = item?.path();
        let is_own = path.extension().is_some_and(|e| e == "desktop")
            && fs::read_to_string(&path).is_ok_and(|t| t.lines().any(|l| l.starts_with(MARKER)));
        if is_own && let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            files.push(name.to_owned());
        }
    }
    Ok(files)
}

fn stem_of(file: &str) -> &str {
    file.strip_suffix(".desktop").unwrap_or(file)
}

/// A file name ES-DE shows well: the title without path separators.
fn file_stem(title: &str) -> String {
    title.replace(['/', '\\'], "-")
}

fn game_path_of(node: &XMLNode) -> Option<String> {
    let game = node.as_element().filter(|e| e.name == "game")?;
    Some(game.get_child("path")?.get_text()?.into_owned())
}

fn text_element(name: &str, text: &str) -> Element {
    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(text.to_owned()));
    element
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
