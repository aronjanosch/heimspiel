use std::fs;
use std::path::{Path, PathBuf};

use heimspiel_core::library::{Entry, FileLocation, Launch, Layout, Library, Metadata, SourceKind};
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
        launch: Launch::Command {
            exec: format!("steam steam://rungameid/{id}"),
            working_dir: None,
        },
        cover: None,
        metadata: Default::default(),
        identity: Default::default(),
    }
}

fn romm(rom_id: u64, platform: &str, file_name: &str, title: &str) -> Entry {
    romm_laid_out(rom_id, platform, file_name, title, Layout::File)
}

fn romm_laid_out(
    rom_id: u64,
    platform: &str,
    file_name: &str,
    title: &str,
    layout: Layout,
) -> Entry {
    Entry {
        source: SourceKind::RomM,
        id: rom_id.to_string(),
        title: title.into(),
        launch: Launch::File {
            platform: platform.into(),
            file_name: file_name.into(),
            location: FileLocation::RomM { rom_id, layout },
        },
        cover: None,
        metadata: Metadata::default(),
        identity: Default::default(),
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
        launch: Launch::Command {
            exec: format!("heroic --no-gui heroic://launch?appName={id}"),
            working_dir: None,
        },
        cover: None,
        metadata: Default::default(),
        identity: Default::default(),
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

#[test]
fn writes_a_placeholder_named_like_the_game_file_with_its_metadata() {
    let s = setup();
    let mut lode = romm(3039, "n64", "Lode Runner 3-D (USA).n64", "Lode Runner 3-D");
    lode.metadata = Metadata {
        description: Some("Outrun monsters.".into()),
        release_date: Some("1999-03-26".into()),
        developer: Some("Big Bang".into()),
        publisher: Some("Infogrames".into()),
        genre: Some("Platform".into()),
        players: Some("1".into()),
        rating: Some(0.635),
    };

    s.esde.write(&library(vec![lode])).unwrap();

    let placeholder = fs::read(s.roms.join("n64/Lode Runner 3-D (USA).n64")).unwrap();
    assert!(placeholder.len() < 100);
    let gamelist = read(s.home.join("gamelists/n64/gamelist.xml"));
    for field in [
        "<path>./Lode Runner 3-D (USA).n64</path>",
        "<name>Lode Runner 3-D</name>",
        "<desc>Outrun monsters.</desc>",
        "<releasedate>19990326T000000</releasedate>",
        "<developer>Big Bang</developer>",
        "<publisher>Infogrames</publisher>",
        "<genre>Platform</genre>",
        "<players>1</players>",
        "<rating>0.6</rating>",
    ] {
        assert!(gamelist.contains(field), "missing {field} in {gamelist}");
    }
}

#[test]
fn puts_platforms_into_the_system_es_de_names_them() {
    let s = setup();

    s.esde
        .write(&library(vec![romm(1, "ngc", "Pikmin (USA).iso", "Pikmin")]))
        .unwrap();

    assert!(s.roms.join("gc/Pikmin (USA).iso").exists());
}

#[test]
fn removes_placeholders_of_games_gone_from_romm_but_never_a_real_game_file() {
    let s = setup();
    s.esde
        .write(&library(vec![
            romm(1, "n64", "Gone (USA).z64", "Gone"),
            romm(2, "n64", "Mine (USA).z64", "Mine"),
        ]))
        .unwrap();
    // The user replaced one Placeholder with the real game file.
    fs::write(s.roms.join("n64/Mine (USA).z64"), vec![0x80u8; 4096]).unwrap();

    s.esde.write(&library(vec![])).unwrap();

    assert!(!s.roms.join("n64/Gone (USA).z64").exists());
    assert_eq!(
        fs::read(s.roms.join("n64/Mine (USA).z64")).unwrap().len(),
        4096
    );
}

#[test]
fn keeps_a_link_to_a_fetched_game_and_removes_it_with_the_game() {
    let s = setup();
    let cache = s.home.parent().unwrap().join("cache");
    let esde = EsDe::locate(&s.home, s.home.parent().unwrap())
        .unwrap()
        .game_cache(&cache);
    let lode = || library(vec![romm(3039, "n64", "Lode.n64", "Lode")]);
    esde.write(&lode()).unwrap();
    let game = s.roms.join("n64/Lode.n64");
    let cached = cache.join("3039/Lode.n64");
    fs::create_dir_all(cached.parent().unwrap()).unwrap();
    fs::write(&cached, b"rom").unwrap();
    fs::remove_file(&game).unwrap();
    std::os::unix::fs::symlink(&cached, &game).unwrap();

    esde.write(&lode()).unwrap();
    assert_eq!(fs::read(&game).unwrap(), b"rom");

    esde.write(&library(vec![])).unwrap();
    assert!(fs::symlink_metadata(&game).is_err());
}

#[test]
fn writes_a_game_of_several_files_as_a_folder_es_de_treats_as_one_game() {
    let s = setup();

    s.esde
        .write(&library(vec![
            romm_laid_out(
                12566,
                "dc",
                "Metal Slug 6.gdi",
                "Metal Slug 6",
                Layout::StartInFolder,
            ),
            romm_laid_out(846, "scummvm", "Tentacle.svm", "Tentacle", Layout::Folder),
        ]))
        .unwrap();

    let gdi = s.roms.join("dreamcast/Metal Slug 6.gdi");
    assert_eq!(
        read(gdi.join("Metal Slug 6.gdi")),
        "heimspiel-placeholder romm:12566\n"
    );
    assert_eq!(
        read(gdi.join(".heimspiel")),
        "heimspiel-placeholder romm:12566\n"
    );
    let svm = s.roms.join("scummvm/Tentacle.svm");
    assert_eq!(fs::read_dir(&svm).unwrap().count(), 1);
    assert_eq!(
        read(svm.join(".heimspiel")),
        "heimspiel-placeholder romm:846\n"
    );
    assert!(
        read(s.home.join("gamelists/dreamcast/gamelist.xml"))
            .contains("<path>./Metal Slug 6.gdi</path>")
    );
}

#[test]
fn keeps_a_fetched_folder_and_removes_it_with_the_game_but_no_foreign_folder() {
    let s = setup();
    let cache = s.home.parent().unwrap().join("cache");
    let esde = EsDe::locate(&s.home, s.home.parent().unwrap())
        .unwrap()
        .game_cache(&cache);
    let metal_slug = || {
        library(vec![romm_laid_out(
            12566,
            "dc",
            "Metal Slug 6.gdi",
            "Metal Slug 6",
            Layout::StartInFolder,
        )])
    };
    esde.write(&metal_slug()).unwrap();
    // The hook linked the game's files into the folder.
    let folder = s.roms.join("dreamcast/Metal Slug 6.gdi");
    let cached = cache.join("12566/disc.gdi");
    fs::create_dir_all(cached.parent().unwrap()).unwrap();
    fs::write(&cached, b"gdi").unwrap();
    fs::remove_file(folder.join("Metal Slug 6.gdi")).unwrap();
    std::os::unix::fs::symlink(&cached, folder.join("Metal Slug 6.gdi")).unwrap();
    std::os::unix::fs::symlink(&cached, folder.join("disc.gdi")).unwrap();
    // A game folder of the user's own.
    let foreign = s.roms.join("dreamcast/Mine.gdi");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("Mine.gdi"), b"gdi").unwrap();

    esde.write(&metal_slug()).unwrap();
    assert_eq!(fs::read(folder.join("Metal Slug 6.gdi")).unwrap(), b"gdi");

    esde.write(&library(vec![])).unwrap();
    assert!(!folder.exists());
    assert!(cached.exists());
    assert!(foreign.join("Mine.gdi").exists());
}

#[test]
fn turns_a_placeholder_into_a_folder_when_the_game_gained_files() {
    let s = setup();
    s.esde
        .write(&library(vec![romm(7, "psx", "Riven.m3u", "Riven")]))
        .unwrap();

    s.esde
        .write(&library(vec![romm_laid_out(
            7,
            "psx",
            "Riven.m3u",
            "Riven",
            Layout::StartInFolder,
        )]))
        .unwrap();

    assert!(s.roms.join("psx/Riven.m3u/Riven.m3u").is_file());
}

#[test]
fn installs_the_game_start_hook_and_turns_on_event_scripts() {
    let s = setup();
    let command = vec!["/opt/heimspiel bin/heimspiel".to_owned()];

    assert!(s.esde.install_hook(&command).unwrap());
    assert!(!s.esde.install_hook(&command).unwrap());

    let script = s.home.join("scripts/game-start/heimspiel");
    let text = read(&script);
    assert!(text.starts_with("#!/bin/sh\n"));
    assert!(text.contains("exec '/opt/heimspiel bin/heimspiel' hook game-start \"$@\""));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        fs::metadata(&script).unwrap().permissions().mode() & 0o111,
        0o111
    );
    let settings = read(s.home.join("settings/es_settings.xml"));
    assert!(settings.contains("<bool name=\"CustomEventScripts\" value=\"true\" />"));
    assert!(settings.contains("<bool name=\"ShowHiddenGames\" value=\"true\" />"));
}

#[test]
fn links_local_games_and_removes_only_the_link() {
    let s = setup();
    let folder = s.home.parent().unwrap().join("Archive");
    let file = folder.join("n64/Super Mario 64 (USA).z64");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"rom").unwrap();
    let esde = EsDe::locate(&s.home, s.home.parent().unwrap())
        .unwrap()
        .local_folder(&folder);
    let mario = Entry {
        source: SourceKind::Local,
        id: file.display().to_string(),
        title: "Super Mario 64".into(),
        launch: Launch::File {
            platform: "n64".into(),
            file_name: "Super Mario 64 (USA).z64".into(),
            location: FileLocation::Local { path: file.clone() },
        },
        cover: None,
        metadata: Metadata::default(),
        identity: Default::default(),
    };

    esde.write(&library(vec![mario.clone()])).unwrap();
    esde.write(&library(vec![mario])).unwrap();

    let link = s.roms.join("n64/Super Mario 64 (USA).z64");
    assert_eq!(fs::read_link(&link).unwrap(), file);
    assert!(
        read(s.home.join("gamelists/n64/gamelist.xml")).contains("<name>Super Mario 64</name>")
    );

    esde.write(&library(vec![])).unwrap();

    assert!(fs::symlink_metadata(&link).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"rom");
}
