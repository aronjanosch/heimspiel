mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{FakeServer, Response};
use heimspiel_core::cache::GameCache;
use heimspiel_core::fetch::Fetch;
use heimspiel_core::launch::{self, PortLaunch};
use heimspiel_core::port::Manifest;
use heimspiel_core::quiver::Quiver;
use heimspiel_core::romm::Romm;
use xxhash_rust::xxh3::xxh3_64;

const Z64: [u8; 8] = [0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4];
const V64: [u8; 8] = [0x37, 0x80, 0x40, 0x12, 2, 1, 4, 3];

fn manifest() -> Manifest {
    Manifest::parse(&format!(
        r#"
id = "testrecomp"
title = "Test: Recompiled"
[quiver]
name = "TestRecomp"
repository = "test/TestRecomp"
asset-filter = "Linux-X64.zip"
[original]
platform = "n64"
igdb-id = 1
region = "USA"
[provide]
kind = "librecomp"
config-dir = "TestRecompiled"
game-id = "test.n64.us.1.0"
xxh3 = "{:#x}"
"#,
        xxh3_64(&Z64)
    ))
    .unwrap()
}

fn romm() -> FakeServer {
    FakeServer::start(|req| match req.path.as_str() {
        "/api/roms/7" => Response::json(
            r#"{"id": 7, "fs_name": "Test (USA).n64", "name": "Test", "fs_size_bytes": 8, "has_multiple_files": false}"#,
        ),
        "/api/roms/7/content/Test%20%28USA%29.n64" => Response {
            status: 200,
            body: V64.to_vec(),
        },
        _ => Response::status(404),
    })
}

/// A stand-in for the Quiver AppImage: logs its arguments; `--download` installs and then hangs
/// like Quiver 3.5.1 does, unless `installs` is false.
fn fake_quiver(dir: &Path, installs: bool) {
    let install = if installs {
        r#"mkdir -p "$here/Apps/$2" && echo build > "$here/Apps/$2/installed-build.txt""#
    } else {
        ":"
    };
    let script = format!(
        "#!/bin/sh\nhere=$(dirname \"$0\")\necho \"$@\" >> \"$here/calls.log\"\n\
         case \"$1\" in\n  --download) {install}; exec sleep 600;;\n  --run) exit 0;;\nesac\n"
    );
    fs::create_dir_all(dir).unwrap();
    let path = dir.join("QuiverLauncher-linux-x64.AppImage");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

struct Setup {
    tmp: tempfile::TempDir,
    quiver_dir: PathBuf,
    config_home: PathBuf,
}

fn setup(installs: bool) -> Setup {
    let tmp = tempfile::tempdir().unwrap();
    let quiver_dir = tmp.path().join("quiver");
    fake_quiver(&quiver_dir, installs);
    let config_home = tmp.path().join("config");
    Setup {
        quiver_dir,
        config_home,
        tmp,
    }
}

fn run(s: &Setup, server: &FakeServer, install_timeout: Duration) -> anyhow::Result<()> {
    let client = Romm::new(&server.url, "rmm_secret").backoff(Duration::ZERO);
    let cache = GameCache::new(&s.tmp.path().join("cache"), 1 << 30);
    let fetch = Fetch {
        client: &client,
        cache: &cache,
        notify: &|_| {},
        notify_from_bytes: u64::MAX,
    };
    let quiver = Quiver::new(&s.quiver_dir);
    let ctx = PortLaunch {
        manifests: &[manifest()],
        quiver: &quiver,
        fetch: &fetch,
        config_home: &s.config_home,
        install_timeout,
    };
    launch::run(&ctx, "testrecomp", 7)
}

fn calls(s: &Setup) -> Vec<String> {
    fs::read_to_string(s.quiver_dir.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn installs_provides_and_runs_a_port_on_its_first_start() {
    let s = setup(true);
    let server = romm();
    let started = Instant::now();

    run(&s, &server, Duration::from_secs(20)).unwrap();

    // Quiver's --download never exits; heimspiel stops it once the build is installed.
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(calls(&s), ["--download TestRecomp", "--run TestRecomp"]);
    let rom = fs::read(s.config_home.join("TestRecompiled/test.n64.us.1.0.z64")).unwrap();
    assert_eq!(rom, Z64);
    let apps = fs::read_to_string(s.quiver_dir.join("apps.json")).unwrap();
    assert!(
        apps.contains(r#""repository": "test/TestRecomp""#),
        "{apps}"
    );
}

#[test]
fn starts_right_away_the_second_time() {
    let s = setup(true);
    let server = romm();
    run(&s, &server, Duration::from_secs(20)).unwrap();
    let requests = server.requests().len();

    run(&s, &server, Duration::from_secs(20)).unwrap();

    assert_eq!(
        calls(&s),
        [
            "--download TestRecomp",
            "--run TestRecomp",
            "--run TestRecomp"
        ]
    );
    assert_eq!(server.requests().len(), requests);
}

#[test]
fn gives_up_on_an_install_that_never_finishes() {
    let s = setup(false);
    let server = romm();

    let err = run(&s, &server, Duration::from_millis(500)).unwrap_err();

    assert!(format!("{err:#}").contains("TestRecomp"), "{err:#}");
    assert_eq!(calls(&s), ["--download TestRecomp"]);
}

#[test]
fn keeps_the_other_apps_in_quivers_library() {
    let s = setup(true);
    fs::write(
        s.quiver_dir.join("apps.json"),
        r#"{"apps": [{"name": "Mine", "repository": "me/Mine", "folderName": "Mine", "releaseAssetFilter": "x.zip"}]}"#,
    )
    .unwrap();
    let server = romm();

    run(&s, &server, Duration::from_secs(20)).unwrap();

    let apps = fs::read_to_string(s.quiver_dir.join("apps.json")).unwrap();
    assert!(apps.contains(r#""repository": "me/Mine""#), "{apps}");
    assert!(
        apps.contains(r#""repository": "test/TestRecomp""#),
        "{apps}"
    );
}
