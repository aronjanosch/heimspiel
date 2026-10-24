mod common;

use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{FakeServer, Response};
use heimspiel_core::cache::GameCache;
use heimspiel_core::hook::{GameStart, game_start};
use heimspiel_core::romm::Romm;

const PLACEHOLDER: &str = "heimspiel-placeholder romm:3039\n";
const DETAIL: &str = r#"{"id": 3039, "fs_name": "Lode Runner 3-D (USA).n64", "name": "Lode Runner 3-D",
    "fs_size_bytes": 3, "has_multiple_files": false}"#;

fn server() -> FakeServer {
    FakeServer::start(|req| match req.path.as_str() {
        "/api/roms/3039" => Response::json(DETAIL),
        "/api/roms/3039/content/Lode%20Runner%203-D%20%28USA%29.n64" => Response {
            status: 200,
            body: b"rom".to_vec(),
        },
        _ => Response::status(404),
    })
}

struct Setup {
    _tmp: tempfile::TempDir,
    roms: PathBuf,
    cache: GameCache,
    notes: RefCell<Vec<String>>,
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
        notes: RefCell::new(Vec::new()),
        _tmp: tmp,
    }
}

fn run(s: &Setup, server: &FakeServer, arg: &str, notify_from_bytes: u64) -> anyhow::Result<()> {
    let client = Romm::new(&server.url, "rmm_secret").backoff(Duration::ZERO);
    let notify = |text: &str| s.notes.borrow_mut().push(text.to_owned());
    let ctx = GameStart {
        client: &client,
        cache: &s.cache,
        notify: &notify,
        notify_from_bytes,
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

#[test]
fn swaps_the_placeholder_for_a_link_to_the_downloaded_game() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    run(&s, &server, &escaped(&game), u64::MAX).unwrap();

    assert_eq!(fs::read(&game).unwrap(), b"rom");
    assert!(
        fs::symlink_metadata(&game)
            .unwrap()
            .file_type()
            .is_symlink()
    );
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

    run(&s, &server, &escaped(&real), u64::MAX).unwrap();
    // ES-DE passes the expanded Exec line for .desktop games.
    run(&s, &server, "steam steam://rungameid/590380", u64::MAX).unwrap();

    assert!(server.requests().is_empty());
    assert_eq!(fs::read(&real).unwrap().len(), 4096);
}

#[test]
fn tells_the_user_about_large_downloads() {
    let s = setup();
    let server = server();
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    run(&s, &server, &escaped(&game), 1).unwrap();

    let notes = s.notes.borrow();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes[0].contains("Lode Runner 3-D"));
}

#[test]
fn keeps_the_placeholder_when_the_download_fails() {
    let s = setup();
    let server = FakeServer::start(|req| match req.path.as_str() {
        "/api/roms/3039" => Response::json(DETAIL),
        _ => Response::status(404),
    });
    let game = s.roms.join("Lode Runner 3-D (USA).n64");

    assert!(run(&s, &server, &escaped(&game), 1).is_err());

    assert_eq!(fs::read_to_string(&game).unwrap(), PLACEHOLDER);
    assert!(s.notes.borrow().last().unwrap().contains("Lode Runner 3-D"));
}
