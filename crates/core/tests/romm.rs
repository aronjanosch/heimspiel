mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{FakeServer, Response};
use heimspiel_core::library::{FileLocation, Launch, SourceKind};
use heimspiel_core::romm::Romm;
use heimspiel_core::source::romm;

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
            location: FileLocation::RomM { rom_id: 1 },
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
