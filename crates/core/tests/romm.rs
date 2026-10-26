mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{FakeServer, Response};
use heimspiel_core::library::{FileLocation, Launch, Layout, SourceKind};
use heimspiel_core::romm::{RomDetail, Romm};
use heimspiel_core::source::romm;
use heimspiel_core::start::Start;

const PLATFORMS: &str = r#"[
    {"id": 52, "slug": "n64", "fs_slug": "n64", "name": "Nintendo 64", "rom_count": 3},
    {"id": 195, "slug": "gba", "fs_slug": "gba", "name": "Game Boy Advance", "rom_count": 1}
]"#;

/// The signature of a PNG file.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

fn rom(id: u64, fs_name: &str, name: &str) -> String {
    format!(
        r#"{{"id": {id}, "platform_id": 52, "platform_slug": "n64", "fs_name": "{fs_name}",
            "name": "{name}", "summary": null, "path_cover_large": null, "metadatum": null}}"#
    )
}

fn page(items: &[String], total: usize, offset: usize) -> String {
    format!(
        r#"{{"items": [{}], "total": {total}, "limit": 2, "offset": {offset}}}"#,
        items.join(",")
    )
}

fn client(server: &FakeServer) -> Romm {
    Romm::new(&server.url, "rmm_secret")
        .page_size(2)
        .backoff(Duration::ZERO)
        .media_interval(Duration::ZERO)
}

#[test]
fn lists_the_games_of_the_chosen_platforms_page_by_page() {
    let server = FakeServer::start(|req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        p if p.starts_with("/api/roms?")
            && p.contains("platform_ids=52")
            && p.contains("offset=0") =>
        {
            Response::json(&page(
                &[rom(1, "A (USA).z64", "A"), rom(2, "B (USA).z64", "B")],
                3,
                0,
            ))
        }
        p if p.starts_with("/api/roms?")
            && p.contains("platform_ids=52")
            && p.contains("offset=2") =>
        {
            Response::json(&page(&[rom(3, "C (USA).z64", "C")], 3, 2))
        }
        _ => Response::status(404),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    let titles: Vec<_> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["A", "B", "C"]);
    let a = &entries[0];
    assert_eq!(a.source, SourceKind::RomM);
    assert_eq!(a.id, "1");
    assert_eq!(
        a.launch,
        Launch::File {
            platform: "n64".into(),
            file_name: "A (USA).z64".into(),
            location: FileLocation::RomM {
                rom_id: 1,
                layout: Layout::File,
            },
        }
    );
    assert!(
        server
            .requests()
            .iter()
            .all(|r| r.authorization.as_deref() == Some("Bearer rmm_secret"))
    );
    assert!(
        !server
            .requests()
            .iter()
            .any(|r| r.path.contains("platform_ids=195"))
    );
}

#[test]
fn maps_romm_metadata() {
    let lode = r#"{"id": 3039, "platform_id": 52, "fs_name": "Lode Runner 3-D (USA).n64", "name": "Lode Runner 3-D",
        "summary": "Outrun monsters.", "path_cover_large": null,
        "metadatum": {"genres": ["Platform", "Puzzle"], "player_count": "1",
                      "first_release_date": 922406400000, "average_rating": 63.5}}"#;
    let body = format!(r#"{{"items": [{lode}], "total": 1, "limit": 2, "offset": 0}}"#);
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        _ => Response::json(&body),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    let m = &entries[0].metadata;
    assert_eq!(m.description.as_deref(), Some("Outrun monsters."));
    assert_eq!(m.release_date.as_deref(), Some("1999-03-26"));
    assert_eq!(m.genre.as_deref(), Some("Platform, Puzzle"));
    assert_eq!(m.players.as_deref(), Some("1"));
    assert_eq!(m.rating, Some(0.635));
}

#[test]
fn skips_save_files_stored_next_to_the_games() {
    let server = FakeServer::start(|req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        _ => Response::json(&page(
            &[
                rom(1, "Mario Kart 64 (U) [!].z64", "MK"),
                rom(2, "Mario Kart 64 (U) [!].eep", "MK"),
                rom(3, "MK.mpk", "MK"),
            ],
            3,
            0,
        )),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    assert_eq!(entries.len(), 1);
}

#[test]
fn downloads_each_cover_once_into_the_cache() {
    let body = r#"{"items": [{"id": 7, "platform_id": 52, "fs_name": "A.z64", "name": "A", "summary": null,
        "metadatum": null, "path_cover_large": "/assets/romm/resources/roms/52/7/cover/big.png?ts=2025-03-27 16:20:24"}],
        "total": 1, "limit": 2, "offset": 0}"#;
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        p if p.starts_with("/assets/romm/resources/roms/52/7/cover/big.png") => Response {
            status: 200,
            body: PNG.to_vec(),
        },
        _ => Response::json(body),
    });
    let tmp = tempfile::tempdir().unwrap();
    let romm_client = client(&server);

    let first = romm::entries(&romm_client, &["n64".into()], tmp.path()).unwrap();
    let second = romm::entries(&romm_client, &["n64".into()], tmp.path()).unwrap();

    let cover = first[0].cover.clone().unwrap();
    assert_eq!(std::fs::read(&cover).unwrap(), PNG);
    assert_eq!(cover.extension().unwrap(), "png");
    assert_eq!(second[0].cover, Some(cover));
    let cover_requests = server
        .requests()
        .iter()
        .filter(|r| r.path.starts_with("/assets/"))
        .count();
    assert_eq!(cover_requests, 1);
}

#[test]
fn retries_a_busy_server() {
    let failures = AtomicUsize::new(0);
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" if failures.fetch_add(1, Ordering::SeqCst) < 2 => Response::status(503),
        "/api/platforms" => Response::json(PLATFORMS),
        _ => Response::json(&page(&[], 0, 0)),
    });
    let tmp = tempfile::tempdir().unwrap();

    assert!(romm::entries(&client(&server), &["n64".into()], tmp.path()).is_ok());
}

#[test]
fn gives_up_with_an_error_that_does_not_reveal_the_token() {
    let server = FakeServer::start(|_| Response::status(503));
    let tmp = tempfile::tempdir().unwrap();

    let err = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap_err();

    let message = format!("{err:#}");
    assert!(message.contains("503"), "{message}");
    assert!(!message.contains("rmm_secret"));
}

#[test]
fn treats_an_empty_cover_path_as_no_cover() {
    let body = r#"{"items": [{"id": 7, "platform_id": 52, "fs_name": "A.z64", "name": "A", "summary": null,
        "metadatum": null, "path_cover_large": ""}], "total": 1, "limit": 2, "offset": 0}"#;
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        p if p.starts_with("/api/roms?") => Response::json(body),
        _ => Response::json("<!doctype html>"),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    assert_eq!(entries[0].cover, None);
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn keeps_no_cover_that_is_not_an_image() {
    let body = r#"{"items": [{"id": 7, "platform_id": 52, "fs_name": "A.z64", "name": "A", "summary": null,
        "metadatum": null, "path_cover_large": "/assets/romm/resources/roms/52/7/cover/big.png"}],
        "total": 1, "limit": 2, "offset": 0}"#;
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        p if p.starts_with("/api/roms?") => Response::json(body),
        // A web frontend answering every unknown path with its index page.
        _ => Response::json("<!doctype html><html></html>"),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    assert_eq!(entries[0].cover, None);
    assert_eq!(
        std::fs::read_dir(tmp.path())
            .map(|d| d.count())
            .unwrap_or(0),
        0
    );
}

#[test]
fn passes_on_what_identifies_the_game() {
    let body = r#"{"items": [
        {"id": 3037, "fs_name": "MM (USA).n64", "name": "MM", "igdb_id": 1030, "regions": ["USA"], "revision": ""},
        {"id": 3038, "fs_name": "OoT (USA) (Rev B).n64", "name": "OoT", "igdb_id": 1029, "regions": ["USA"], "revision": "B"}
    ], "total": 2, "limit": 2, "offset": 0}"#;
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        _ => Response::json(body),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    assert_eq!(entries[0].identity.igdb_id, Some(1030));
    assert_eq!(entries[0].identity.regions, ["USA"]);
    assert_eq!(entries[0].identity.revision, None);
    assert_eq!(entries[1].identity.revision.as_deref(), Some("B"));
}

/// A game in the RomM folder `roms/<platform>/<folder>` made of these files.
fn several(folder: &str, extension: &str, files: &[&str]) -> RomDetail {
    let files: Vec<String> = files
        .iter()
        .enumerate()
        .map(|(id, name)| {
            let (dir, name) = match name.rsplit_once('/') {
                Some((dir, name)) => (format!("roms/psx/{folder}/{dir}"), name),
                None => (format!("roms/psx/{folder}"), *name),
            };
            format!(
                r#"{{"id": {id}, "file_name": "{name}", "file_path": "{dir}", "file_size_bytes": 1}}"#
            )
        })
        .collect();
    serde_json::from_str(&format!(
        r#"{{"id": 1, "fs_name": "{folder}", "fs_extension": "{extension}", "fs_path": "roms/psx",
            "name": "Game", "has_multiple_files": true, "files": [{}]}}"#,
        files.join(",")
    ))
    .unwrap()
}

fn file(name: &str) -> Option<Start> {
    Some(Start::File(name.into()))
}

#[test]
fn tells_what_to_start_of_a_game_made_of_several_files() {
    let cue = several(
        "FF7",
        "",
        &["FF7 (Track 2).bin", "FF7.cue", "FF7 (Track 1).bin"],
    );
    assert_eq!(cue.start(), file("FF7.cue"));
    let playlist = several(
        "FF7",
        "",
        &["FF7 (Disc 1).cue", "FF7.m3u", "FF7 (Disc 2).cue"],
    );
    assert_eq!(playlist.start(), file("FF7.m3u"));
    let gdi = several("MS6", "", &["track01.iso", "disc.gdi", "track02.raw"]);
    assert_eq!(gdi.start(), file("disc.gdi"));
    let discs = several("Riven", "", &["Riven (Disc 2).chd", "Riven (Disc 1).chd"]);
    assert_eq!(
        discs.start(),
        Some(Start::Playlist(vec![
            "Riven (Disc 1).chd".into(),
            "Riven (Disc 2).chd".into()
        ]))
    );
    let msu1 = several(
        "Actraiser (MSU-1)",
        "",
        &["a-1.pcm", "a.sfc", "a.msu", "a-2.pcm"],
    );
    assert_eq!(msu1.start(), file("a.sfc"));
    let svm = several("Tentacle.svm", "svm", &["tentacle.000", "monster.sou"]);
    assert_eq!(svm.start(), Some(Start::Folder));
    // RomM takes `1)` for the extension of `Game (v1.1)`.
    let version = several("Game (v1.1)", "1)", &["Game.cue", "Game.bin"]);
    assert_eq!(version.start(), file("Game.cue"));
    let artwork = several("boxart", "", &["ace.png", "astron.png"]);
    assert_eq!(artwork.start(), None);
}

#[test]
fn keeps_the_subfolders_of_a_game_and_nothing_outside_it() {
    let game = several("Cave Story", "", &["Doukutsu.exe", "data/Stage/0.pxa"]);
    let paths: Vec<String> = game
        .files
        .iter()
        .map(|f| game.relative_path(f).unwrap())
        .collect();
    assert_eq!(paths, ["Doukutsu.exe", "data/Stage/0.pxa"]);

    let escaping = several("Game", "", &["../../../etc/passwd"]);
    assert!(escaping.relative_path(&escaping.files[0]).is_err());
}

#[test]
fn places_a_game_of_several_files_as_a_folder_named_after_what_starts() {
    let items = r#"{"items": [
        {"id": 1, "fs_name": "Metal Slug 6", "name": "Metal Slug 6", "has_multiple_files": true},
        {"id": 2, "fs_name": "boxart", "name": "boxart", "has_multiple_files": true},
        {"id": 3, "fs_name": "Tentacle.svm", "name": "Tentacle", "has_multiple_files": true}
    ], "total": 3, "limit": 2, "offset": 0}"#;
    let detail = |id: u64, fs_name: &str, ext: &str, files: &str| {
        format!(
            r#"{{"id": {id}, "fs_name": "{fs_name}", "fs_extension": "{ext}", "fs_path": "roms/dc",
                "name": "x", "has_multiple_files": true, "files": [{files}]}}"#
        )
    };
    let in_folder = |folder: &str, name: &str| {
        format!(
            r#"{{"id": 9, "file_name": "{name}", "file_path": "roms/dc/{folder}", "file_size_bytes": 1}}"#
        )
    };
    let metal_slug = detail(
        1,
        "Metal Slug 6",
        "",
        &[
            in_folder("Metal Slug 6", "disc.gdi"),
            in_folder("Metal Slug 6", "track01.iso"),
        ]
        .join(","),
    );
    let boxart = detail(2, "boxart", "", &in_folder("boxart", "ace.png"));
    let svm = detail(
        3,
        "Tentacle.svm",
        "svm",
        &in_folder("Tentacle.svm", "a.000"),
    );
    let server = FakeServer::start(move |req| match req.path.as_str() {
        "/api/platforms" => Response::json(PLATFORMS),
        "/api/roms/1" => Response::json(&metal_slug),
        "/api/roms/2" => Response::json(&boxart),
        "/api/roms/3" => Response::json(&svm),
        _ => Response::json(items),
    });
    let tmp = tempfile::tempdir().unwrap();

    let entries = romm::entries(&client(&server), &["n64".into()], tmp.path()).unwrap();

    let placed: Vec<_> = entries
        .iter()
        .map(|e| match &e.launch {
            Launch::File {
                file_name,
                location: FileLocation::RomM { layout, .. },
                ..
            } => (file_name.as_str(), *layout),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        placed,
        [
            ("Metal Slug 6.gdi", Layout::StartInFolder),
            ("Tentacle.svm", Layout::Folder)
        ]
    );
}
