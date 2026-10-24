use std::fs;
use std::path::{Path, PathBuf};

use heimspiel_core::library::{Entry, Library, SourceKind};
use heimspiel_core::output::esde::EsDe;

struct Setup {
    _tmp: tempfile::TempDir,
    esde: EsDe,
    home: PathBuf,
    roms: PathBuf,
}

/// An ES-DE home whose settings point the ROM folder elsewhere, as users often do.
fn setup() -> Setup {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("ES-DE");
    let roms = tmp.path().join("Games/ROMs");
    fs::create_dir_all(home.join("settings")).unwrap();
    fs::write(
        home.join("settings/es_settings.xml"),
        format!(
            "<?xml version=\"1.0\"?>\n<bool name=\"ShowHiddenGames\" value=\"true\" />\n\
             <string name=\"MediaDirectory\" value=\"\" />\n\
             <string name=\"ROMDirectory\" value=\"{}\" />\n",
            roms.display()
        ),
    )
    .unwrap();
    let esde = EsDe::locate(&home, tmp.path()).unwrap();
    Setup {
        _tmp: tmp,
        esde,
        home,
        roms,
    }
}

fn steam(id: &str, title: &str) -> Entry {
    Entry {
        source: SourceKind::Steam,
        id: id.into(),
        title: title.into(),
        exec: format!("steam steam://rungameid/{id}"),
        working_dir: None,
        cover: None,
    }
}

fn library(entries: Vec<Entry>) -> Library {
    Library { entries }
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn writes_a_launchable_file_and_a_gamelist_entry_per_entry() {
    let s = setup();

    s.esde
        .write(&library(vec![steam("590380", "Into the Breach")]))
        .unwrap();

    let file = read(s.roms.join("steam/Into the Breach.desktop"));
    assert!(file.contains("Name=Into the Breach\n"));
    assert!(file.contains("Exec=steam steam://rungameid/590380\n"));
    let gamelist = read(s.home.join("gamelists/steam/gamelist.xml"));
    assert!(gamelist.contains("<path>./Into the Breach.desktop</path>"));
    assert!(gamelist.contains("<name>Into the Breach</name>"));
}

#[test]
fn a_second_sync_keeps_what_es_de_wrote_and_touches_no_file() {
    let s = setup();
    let lib = library(vec![steam("590380", "Into the Breach")]);
    s.esde.write(&lib).unwrap();
    let gamelist_path = s.home.join("gamelists/steam/gamelist.xml");
    // ES-DE rewrites the gamelist in its own layout after a game was played and renamed.
    fs::write(
        &gamelist_path,
        "<?xml version=\"1.0\"?>\n<alternativeEmulator>\n\t<label>Steam</label>\n</alternativeEmulator>\n\
         <gameList>\n\t<game>\n\t\t<path>./Into the Breach.desktop</path>\n\t\t<name>Breach</name>\n\
         \t\t<favorite>true</favorite>\n\t\t<playcount>3</playcount>\n\t</game>\n</gameList>\n",
    )
    .unwrap();
    let before = fs::read(&gamelist_path).unwrap();
    let file_mtime = fs::metadata(s.roms.join("steam/Into the Breach.desktop"))
        .unwrap()
        .modified()
        .unwrap();

    s.esde.write(&lib).unwrap();

    assert_eq!(fs::read(&gamelist_path).unwrap(), before);
    let file_mtime_after = fs::metadata(s.roms.join("steam/Into the Breach.desktop"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(file_mtime_after, file_mtime);
}

#[test]
fn removes_its_own_entries_that_left_the_library_and_keeps_foreign_ones() {
    let s = setup();
    s.esde
        .write(&library(vec![
            steam("590380", "Into the Breach"),
            steam("570", "Dota 2"),
        ]))
        .unwrap();
    // A file the user copied in by hand, with its gamelist entry.
    fs::write(
        s.roms.join("steam/Mine.desktop"),
        "[Desktop Entry]\nName=Mine\nExec=mine\n",
    )
    .unwrap();
    let gamelist_path = s.home.join("gamelists/steam/gamelist.xml");
    let gamelist = read(&gamelist_path).replace(
        "</gameList>",
        "\t<game>\n\t\t<path>./Mine.desktop</path>\n\t\t<name>Mine</name>\n\t</game>\n</gameList>",
    );
    fs::write(&gamelist_path, gamelist).unwrap();

    s.esde
        .write(&library(vec![steam("590380", "Into the Breach")]))
        .unwrap();

    assert!(!s.roms.join("steam/Dota 2.desktop").exists());
    assert!(s.roms.join("steam/Into the Breach.desktop").exists());
    assert!(s.roms.join("steam/Mine.desktop").exists());
    let gamelist = read(&gamelist_path);
    assert!(!gamelist.contains("Dota 2"));
    assert!(gamelist.contains("<path>./Into the Breach.desktop</path>"));
    assert!(gamelist.contains("<path>./Mine.desktop</path>"));
}

#[test]
fn removes_its_entries_from_a_system_that_became_empty() {
    let s = setup();
    s.esde
        .write(&library(vec![steam("570", "Dota 2")]))
        .unwrap();

    s.esde.write(&library(vec![])).unwrap();

    assert!(!s.roms.join("steam/Dota 2.desktop").exists());
    assert!(!read(s.home.join("gamelists/steam/gamelist.xml")).contains("Dota 2"));
}

#[test]
fn leaves_a_foreign_file_with_the_same_name_untouched() {
    let s = setup();
    // ES-DE's own Game Importer copied Steam's shortcut here before.
    let foreign =
        "[Desktop Entry]\nName=Dota 2\nExec=steam steam://rungameid/570\nIcon=steam_icon_570\n";
    fs::create_dir_all(s.roms.join("steam")).unwrap();
    fs::write(s.roms.join("steam/Dota 2.desktop"), foreign).unwrap();

    s.esde
        .write(&library(vec![steam("570", "Dota 2")]))
        .unwrap();
    s.esde.write(&library(vec![])).unwrap();

    assert_eq!(read(s.roms.join("steam/Dota 2.desktop")), foreign);
    let files = fs::read_dir(s.roms.join("steam")).unwrap().count();
    assert_eq!(files, 1);
}

#[test]
fn gives_entries_with_the_same_title_their_own_files() {
    let s = setup();
    let heroic = |id: &str| Entry {
        source: SourceKind::Heroic,
        id: id.into(),
        title: "Fallout".into(),
        exec: format!("heroic --no-gui heroic://launch?appName={id}"),
        working_dir: None,
        cover: None,
    };
    let lib = library(vec![heroic("gog:1"), heroic("legendary:2")]);

    s.esde.write(&lib).unwrap();
    s.esde.write(&lib).unwrap();

    let mut files: Vec<_> = fs::read_dir(s.roms.join("epic"))
        .unwrap()
        .map(|f| f.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, ["Fallout (2).desktop", "Fallout.desktop"]);
    let gamelist = read(s.home.join("gamelists/epic/gamelist.xml"));
    assert_eq!(gamelist.matches("<game>").count(), 2);
}

#[test]
fn places_covers_in_the_media_folder_and_removes_them_with_the_entry() {
    let s = setup();
    let cover = s.home.join("cache-cover.jpg");
    fs::write(&cover, b"jpg").unwrap();
    let mut breach = steam("590380", "Into the Breach");
    breach.cover = Some(cover);

    s.esde.write(&library(vec![breach])).unwrap();

    let placed = s
        .home
        .join("downloaded_media/steam/covers/Into the Breach.jpg");
    assert_eq!(fs::read(&placed).unwrap(), b"jpg");

    s.esde.write(&library(vec![])).unwrap();

    assert!(!placed.exists());
}
