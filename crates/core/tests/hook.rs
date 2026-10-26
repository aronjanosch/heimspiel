mod common;

use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{FakeServer, Request, Response};
use heimspiel_core::cache::GameCache;
use heimspiel_core::fetch::Fetch;
use heimspiel_core::hook::game_start;
use heimspiel_core::progress::Progress;
use heimspiel_core::romm::Romm;

const PLACEHOLDER: &str = "heimspiel-placeholder romm:3039\n";
const DETAIL: &str = r#"{"id": 3039, "fs_name": "Lode Runner 3-D (USA).n64", "fs_path": "roms/n64",
    "name": "Lode Runner 3-D", "has_multiple_files": false,
    "files": [{"id": 1, "file_name": "Lode Runner 3-D (USA).n64", "file_path": "roms/n64",
               "file_size_bytes": 3}]}"#;
const CONTENT: &str = "/api/roms/1/files/content/Lode%20Runner%203-D%20%28USA%29.n64";

/// Metal Slug 6 as RomM keeps it: a folder with a GDI and its tracks.
const GDI_DETAIL: &str = r#"{"id": 12566, "fs_name": "Metal Slug 6", "fs_path": "roms/dc",
    "fs_extension": "", "name": "Metal Slug 6", "has_multiple_files": true, "files": [
    {"id": 11, "file_name": "disc.gdi", "file_path": "roms/dc/Metal Slug 6", "file_size_bytes": 3},
    {"id": 12, "file_name": "track01.iso", "file_path": "roms/dc/Metal Slug 6", "file_size_bytes": 2}]}"#;

/// A two-disc game without a playlist.
const DISCS_DETAIL: &str = r#"{"id": 500, "fs_name": "Riven (USA)", "fs_path": "roms/psx",
    "fs_extension": "", "name": "Riven", "has_multiple_files": true, "files": [
    {"id": 52, "file_name": "Riven (USA) (Disc 2).chd", "file_path": "roms/psx/Riven (USA)", "file_size_bytes": 2},
    {"id": 51, "file_name": "Riven (USA) (Disc 1).chd", "file_path": "roms/psx/Riven (USA)", "file_size_bytes": 2}]}"#;

/// A ScummVM game: a folder that is started as a whole, with a subfolder.
const SVM_DETAIL: &str = r#"{"id": 846, "fs_name": "Tentacle.svm", "fs_path": "roms/scummvm",
    "fs_extension": "svm", "name": "Day of the Tentacle", "has_multiple_files": true, "files": [
    {"id": 61, "file_name": "tentacle.000", "file_path": "roms/scummvm/Tentacle.svm", "file_size_bytes": 2},
    {"id": 62, "file_name": "monster.sou", "file_path": "roms/scummvm/Tentacle.svm/audio", "file_size_bytes": 2}]}"#;

fn server() -> FakeServer {
    FakeServer::start(|req| match req.path.as_str() {
        "/api/roms/3039" => Response::json(DETAIL),
        CONTENT => file(req, b"rom"),
        "/api/roms/12566" => Response::json(GDI_DETAIL),
        "/api/roms/11/files/content/disc.gdi" => file(req, b"gdi"),
        "/api/roms/12/files/content/track01.iso" => file(req, b"t1"),
        "/api/roms/500" => Response::json(DISCS_DETAIL),
        p if p.starts_with("/api/roms/51/") => file(req, b"d1"),
        p if p.starts_with("/api/roms/52/") => file(req, b"d2"),
        "/api/roms/846" => Response::json(SVM_DETAIL),
        "/api/roms/61/files/content/tentacle.000" => file(req, b"t0"),
        "/api/roms/62/files/content/monster.sou" => file(req, b"ms"),
        _ => Response::status(404),
    })
}

/// A file download that honours `Range: bytes=<from>-`.
fn file(req: &Request, body: &[u8]) -> Response {
    let from = req
        .range
        .as_deref()
        .and_then(|r| r.strip_prefix("bytes=")?.strip_suffix('-')?.parse().ok());
    match from {
        Some(from) => Response {
            status: 206,
            body: body[from..].to_vec(),
        },
        None => Response {
            status: 200,
            body: body.to_vec(),
        },
    }
}

/// Records what the user would see.
#[derive(Default)]
struct Shown(RefCell<Vec<String>>);

impl Progress for Shown {
    fn start(&self, game: &str, _cover: Option<&Path>, action: &str) {
        self.0.borrow_mut().push(format!("{action} {game}"));
    }
    fn advance(&self, done: u64, total: u64) {
        self.0.borrow_mut().push(format!("{done}/{total}"));
    }
    fn finish(&self) {
        self.0.borrow_mut().push("finish".into());
    }
    fn fail(&self, message: &str) {
        self.0.borrow_mut().push(format!("fail {message}"));
    }
}

struct Setup {
    tmp: tempfile::TempDir,
    roms: PathBuf,
    cache: GameCache,
    shown: Shown,
}

fn setup() -> Setup {
    let tmp = tempfile::tempdir().unwrap();
    let roms = tmp.path().join("ROMs/n64");
    fs::create_dir_all(&roms).unwrap();
    fs::write(roms.join("Lode Runner 3-D (USA).n64"), PLACEHOLDER).unwrap();
    let cache = GameCache::new(&tmp.path().join("cache"), 1 << 30);
    Setup {
        roms,
        cache,
        shown: Shown::default(),
        tmp,
    }
}

/// A folder as `heimspiel sync` writes it for a game made of several files; with `start`, also
/// the Placeholder named like the folder. Returns the path ES-DE passes to the hook.
fn folder(s: &Setup, system: &str, name: &str, rom_id: u64, start: bool) -> PathBuf {
    let dir = s.tmp.path().join("ROMs").join(system).join(name);
    fs::create_dir_all(&dir).unwrap();
    let placeholder = format!("heimspiel-placeholder romm:{rom_id}\n");
    fs::write(dir.join(".heimspiel"), &placeholder).unwrap();
    if !start {
        return dir;
    }
    fs::write(dir.join(name), &placeholder).unwrap();
    dir.join(name)
}

fn run(s: &Setup, server: &FakeServer, arg: &str, show_from_bytes: u64) -> anyhow::Result<()> {
    let client = Romm::new(&server.url, "rmm_secret").backoff(Duration::ZERO);
    let ctx = Fetch {
        client: &client,
        cache: &s.cache,
        covers: s.tmp.path().join("covers"),
        progress: &s.shown,
        show_from_bytes,
    };
    // ES-DE passes the path shell-escaped, followed by name, system and system name.
    game_start(
        &ctx,
        &[
            arg.to_owned(),
            "Lode Runner 3-D".into(),
            "n64".into(),
            "Nintendo 64".into(),
        ],
    )
}

fn escaped(path: &Path) -> String {
    path.display()
        .to_string()
        .replace(' ', "\\ ")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).unwrap().file_type().is_symlink()
}

#[test]
fn swaps_the_placeholder_for_a_link_to_the_downloaded_game() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(fs::read(&game).unwrap(), b"rom");
    assert!(is_link(&game));
}

#[test]
fn starts_a_cached_game_without_downloading_again() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");
    run(&s, &server, &escaped(&game), u64::MAX).unwrap();
    let requests = server.requests().len();

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(server.requests().len(), requests);
}

#[test]
fn fetches_a_game_again_after_the_cache_dropped_it() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");
    fs::remove_file(&game).unwrap();
    symlink(s.cache.path(3039, "Lode Runner 3-D (USA).n64"), &game).unwrap();

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(fs::read(&game).unwrap(), b"rom");
}

#[test]
fn leaves_every_other_game_alone_without_asking_romm() {
    let s = setup();
    let server = server();
    let real = s.roms.join("Real Game.z64");
    fs::write(&real, vec![0x80u8; 4096]).unwrap();
    let folder = s.tmp.path().join("ROMs/ps3/Real Game.ps3");
    fs::create_dir_all(&folder).unwrap();
    // A local game of several files, as `heimspiel sync` links it.
    let local = s.tmp.path().join("Archive/psx/Crash/Crash.cue");
    fs::create_dir_all(local.parent().unwrap()).unwrap();
    fs::write(&local, b"cue").unwrap();
    let linked = s.tmp.path().join("ROMs/psx/Crash.cue");
    fs::create_dir_all(&linked).unwrap();
    fs::write(linked.join(".heimspiel"), "heimspiel-placeholder local\n").unwrap();
    symlink(&local, linked.join("Crash.cue")).unwrap();

    run(&s, &server, &escaped(&real), u64::MAX).unwrap();
    run(&s, &server, &escaped(&folder), u64::MAX).unwrap();
    run(&s, &server, &escaped(&linked.join("Crash.cue")), u64::MAX).unwrap();
    // ES-DE passes the expanded Exec line for .desktop games.
    run(&s, &server, "steam steam://rungameid/590380", u64::MAX).unwrap();

    assert!(server.requests().is_empty());
    assert_eq!(fs::read(&real).unwrap().len(), 4096);
}

#[test]
fn shows_large_downloads_until_they_are_done() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    run(&s, &server, &escaped(&game), 1).unwrap();

    let shown = s.shown.0.borrow();
    assert_eq!(shown.first().unwrap(), "Downloading Lode Runner 3-D");
    assert!(shown.contains(&"3/3".to_owned()), "{shown:?}");
    assert_eq!(shown.last().unwrap(), "finish");
}

#[test]
fn keeps_the_placeholder_when_the_download_fails() {
    let s = setup();
    let server = FakeServer::start(|req| match req.path.as_str() {
        "/api/roms/3039" => Response::json(DETAIL),
        _ => Response::status(404),
    });
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    let error = run(&s, &server, &escaped(&game), 1).unwrap_err();

    assert_eq!(fs::read_to_string(&game).unwrap(), PLACEHOLDER);
    assert!(format!("{error:#}").contains("Lode Runner 3-D"));
}

#[test]
fn goes_on_with_a_download_that_was_cut_short() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");
    let part = s
        .tmp
        .path()
        .join("cache/3039/Lode Runner 3-D (USA).n64.part");
    fs::create_dir_all(part.parent().unwrap()).unwrap();
    fs::write(&part, b"ro").unwrap();

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(fs::read(&game).unwrap(), b"rom");
    let download = server.requests().into_iter().find(|r| r.path == CONTENT);
    assert_eq!(download.unwrap().range.as_deref(), Some("bytes=2-"));
}

#[test]
fn links_a_disc_and_its_tracks_into_its_folder() {
    let s = setup();
    let server = server();
    let game = folder(&s, "dreamcast", "Metal Slug 6.gdi", 12566, true);

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    // The emulator opens the folder's GDI and finds the tracks next to it.
    assert_eq!(fs::read(&game).unwrap(), b"gdi");
    assert!(is_link(&game));
    let dir = game.parent().unwrap();
    assert_eq!(fs::read(dir.join("track01.iso")).unwrap(), b"t1");
    assert_eq!(fs::read(dir.join("disc.gdi")).unwrap(), b"gdi");

    let requests = server.requests().len();
    run(&s, &server, &escaped(&game), u64::MAX).unwrap();
    assert_eq!(server.requests().len(), requests);
}

#[test]
fn starts_several_discs_through_a_playlist() {
    let s = setup();
    let server = server();
    let game = folder(&s, "psx", "Riven (USA).m3u", 500, true);

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(
        fs::read_to_string(&game).unwrap(),
        "Riven (USA) (Disc 1).chd\nRiven (USA) (Disc 2).chd\n"
    );
    let dir = game.parent().unwrap();
    assert_eq!(
        fs::read(dir.join("Riven (USA) (Disc 2).chd")).unwrap(),
        b"d2"
    );
}

#[test]
fn fills_a_folder_that_is_started_as_a_whole() {
    let s = setup();
    let server = server();
    let game = folder(&s, "scummvm", "Tentacle.svm", 846, false);

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(fs::read(game.join("tentacle.000")).unwrap(), b"t0");
    assert_eq!(fs::read(game.join("audio/monster.sou")).unwrap(), b"ms");

    let requests = server.requests().len();
    run(&s, &server, &escaped(&game), u64::MAX).unwrap();
    assert_eq!(server.requests().len(), requests);
}

#[test]
fn gives_games_the_cache_dropped_their_placeholders_back() {
    let s = setup();
    let server = server();
    let dropped = s.tmp.path().join("ROMs/psx/Dropped.chd");
    fs::create_dir_all(dropped.parent().unwrap()).unwrap();
    symlink(s.cache.path(77, "Dropped.chd"), &dropped).unwrap();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(
        fs::read_to_string(&dropped).unwrap(),
        "heimspiel-placeholder romm:77\n"
    );
    assert!(!s.tmp.path().join("cache/77").exists());
}
