mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{FakeServer, Response};
use heimspiel_core::cache::GameCache;
use heimspiel_core::fetch::Fetch;
use heimspiel_core::launch::{self, OriginalLocation, PortLaunch};
use heimspiel_core::port::Manifest;
use heimspiel_core::proton::Proton;
use heimspiel_core::provision::Homes;
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
releases = [{{ region = "USA" }}]
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

/// When the fake Quiver's `--download` installs.
#[derive(Clone, Copy)]
enum Installs {
    Always,
    /// Like apps in Quiver's own catalog: only the version the catalog lists, `0.9`; for any other
    /// it finds no download and exits. The catalog lands in Quiver's cache on its first run.
    CatalogVersion,
    /// Hangs without installing.
    Never,
}

/// A stand-in for the Quiver AppImage: logs its arguments; `--download` installs and then hangs
/// like Quiver 3.5.1 does.
fn fake_quiver(dir: &Path, installs: Installs) {
    // Copies a stand-in for the Port's program or release, if the test provides one.
    let install = r#"mkdir -p "$here/Apps/$2" && { [ ! -f "$here/port.sh" ] || cp "$here/port.sh" "$here/Apps/$2/port.appimage"; } && { [ ! -d "$here/release" ] || cp -r "$here/release/." "$here/Apps/$2/"; } && echo build > "$here/Apps/$2/installed-build.txt""#;
    let install = match installs {
        Installs::Always => install.to_owned(),
        Installs::CatalogVersion => format!(
            r#"mkdir -p "$here/Cache" && echo '[{{"repository": "test/TestRecomp", "verified": {{"version": "0.9"}}}}]' > "$here/Cache/catalog-release-status.json"; grep -q '"preferredVersion": "0.9"' "$here/apps.json" || {{ echo "No eligible download was found."; exit 1; }}; {install}"#
        ),
        Installs::Never => ":".to_owned(),
    };
    let script = format!(
        "#!/bin/sh\nhere=$(dirname \"$0\")\necho \"$@\" >> \"$here/calls.log\"\n\
         case \"$1\" in\n  --download) {install}; exec sleep 600;;\n  --run) echo \"$SHIP_HOME\" > \"$here/ship_home\"; [ -f \"$here/hangs\" ] && exec sleep 600; exit 0;;\nesac\n"
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

fn setup(installs: Installs) -> Setup {
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
    run_port(s, server, &manifest(), install_timeout)
}

fn run_port(
    s: &Setup,
    server: &FakeServer,
    manifest: &Manifest,
    install_timeout: Duration,
) -> anyhow::Result<()> {
    launch_with(
        s,
        server,
        manifest,
        &OriginalLocation::RomM(7),
        None,
        install_timeout,
    )
}

fn launch_with(
    s: &Setup,
    server: &FakeServer,
    manifest: &Manifest,
    original: &OriginalLocation,
    game: Option<&str>,
    install_timeout: Duration,
) -> anyhow::Result<()> {
    with_ctx(s, server, manifest, install_timeout, &[], |ctx| {
        launch::run(ctx, &manifest.id, original, game)
    })
}

fn with_ctx<T>(
    s: &Setup,
    server: &FakeServer,
    manifest: &Manifest,
    install_timeout: Duration,
    wrapper: &[String],
    f: impl FnOnce(&PortLaunch) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let client = Romm::new(&server.url, "rmm_secret").backoff(Duration::ZERO);
    let cache = GameCache::new(&s.tmp.path().join("cache"), 1 << 30);
    let fetch = Fetch {
        client: &client,
        cache: &cache,
        notify: &|_| {},
        notify_from_bytes: u64::MAX,
    };
    let quiver = Quiver::new(&s.quiver_dir);
    let homes = Homes {
        config: s.config_home.clone(),
        data: s.tmp.path().join("data"),
    };
    // A stand-in for Proton: logs its arguments and the environment it was given.
    let proton_dir = s.tmp.path().join("proton");
    fs::create_dir_all(&proton_dir).unwrap();
    let proton_exe = proton_dir.join("proton");
    fs::write(
        &proton_exe,
        "#!/bin/sh\n{ echo \"$@\"; env | grep -E '^(STEAM_COMPAT|PROTON|SteamGameId)' | sort; } > \"$(dirname \"$0\")/calls.log\"\n",
    )
    .unwrap();
    fs::set_permissions(&proton_exe, fs::Permissions::from_mode(0o755)).unwrap();
    let proton = Proton {
        exe: proton_exe,
        steam_root: s.tmp.path().join("steam"),
    };
    let ctx = PortLaunch {
        manifests: std::slice::from_ref(manifest),
        quiver: &quiver,
        proton: Some(&proton),
        fetch: Some(&fetch),
        notify: &|_| {},
        homes: &homes,
        install_timeout,
        wrapper,
        latest_version: &|_| Ok("1.0".into()),
    };
    f(&ctx)
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
    let s = setup(Installs::Always);
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
    // heimspiel, not Quiver, decides when a Port is updated.
    assert!(apps.contains(r#""preferredVersion": "1.0""#), "{apps}");
}

#[test]
fn pins_the_version_quivers_catalog_lists() {
    let s = setup(Installs::CatalogVersion);
    let server = romm();

    run(&s, &server, Duration::from_secs(20)).unwrap();

    assert_eq!(
        calls(&s),
        [
            "--download TestRecomp",
            "--download TestRecomp",
            "--run TestRecomp"
        ]
    );
    let apps = fs::read_to_string(s.quiver_dir.join("apps.json")).unwrap();
    assert!(apps.contains(r#""preferredVersion": "0.9""#), "{apps}");

    // Once the catalog is known, the first try already pins its version.
    let again = setup(Installs::CatalogVersion);
    fs::create_dir_all(again.quiver_dir.join("Cache")).unwrap();
    fs::copy(
        s.quiver_dir.join("Cache/catalog-release-status.json"),
        again.quiver_dir.join("Cache/catalog-release-status.json"),
    )
    .unwrap();
    run(&again, &server, Duration::from_secs(20)).unwrap();
    assert_eq!(calls(&again), ["--download TestRecomp", "--run TestRecomp"]);
}

#[test]
fn starts_right_away_the_second_time() {
    let s = setup(Installs::Always);
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
    let s = setup(Installs::Never);
    let server = romm();

    let err = run(&s, &server, Duration::from_millis(500)).unwrap_err();

    assert!(format!("{err:#}").contains("TestRecomp"), "{err:#}");
    assert_eq!(calls(&s), ["--download TestRecomp"]);
}

#[test]
fn keeps_the_other_apps_in_quivers_library() {
    let s = setup(Installs::Always);
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

/// Sets up a Port of kind o2r like Ship of Harkinian and launches it inside `wrapper`.
fn launch_o2r_port(wrapper: &[String]) -> (Setup, PathBuf) {
    use sha1::{Digest, Sha1};
    let s = setup(Installs::Always);
    let archive = s.tmp.path().join("data/test/test.o2r");
    // Like SoH: extracts the ROM it is given, then waits at "Run?".
    let port = s.quiver_dir.join("port.sh");
    fs::write(
        &port,
        format!(
            "#!/bin/sh\nmkdir -p \"$(dirname '{a}')\"\ncp \"$1\" '{a}'\necho Done!\nexec sleep 600\n",
            a = archive.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&port, fs::Permissions::from_mode(0o755)).unwrap();
    let manifest = Manifest::parse(&format!(
        r#"
id = "testship"
title = "Test Ship"
[quiver]
name = "TestShip"
repository = "test/TestShip"
asset-filter = ".appimage"
[original]
platform = "n64"
igdb-id = 1
releases = [{{ region = "USA" }}]
[provide]
kind = "o2r"
archive = "test/test.o2r"
sha1 = ["{:x}"]
"#,
        Sha1::digest(Z64)
    ))
    .unwrap();
    let server = romm();

    with_ctx(
        &s,
        &server,
        &manifest,
        Duration::from_secs(20),
        wrapper,
        |ctx| launch::run(ctx, &manifest.id, &OriginalLocation::RomM(7), None),
    )
    .unwrap();
    (s, archive)
}

#[test]
fn extracts_an_o2r_original_with_the_program_quiver_installed() {
    let (s, archive) = launch_o2r_port(&[]);

    assert_eq!(fs::read(&archive).unwrap(), Z64);
    assert_eq!(calls(&s), ["--download TestShip", "--run TestShip"]);
    // libultraship Ports keep their data where heimspiel put the archive, not in Quiver's folder.
    assert_eq!(
        fs::read_to_string(s.quiver_dir.join("ship_home"))
            .unwrap()
            .trim(),
        archive.parent().unwrap().to_string_lossy()
    );
}

#[test]
fn runs_the_extraction_inside_the_wrapper_too() {
    let tmp = tempfile::tempdir().unwrap();
    let wrapped = tmp.path().join("wrapped.log");
    let wrapper = [
        "sh".to_owned(),
        "-c".to_owned(),
        format!(
            "echo \"$(basename \"$0\") $1\" >> '{}'; exec \"$0\" \"$@\"",
            wrapped.display()
        ),
    ];

    let (_s, archive) = launch_o2r_port(&wrapper);

    assert_eq!(fs::read(&archive).unwrap(), Z64);
    let log = fs::read_to_string(&wrapped).unwrap();
    let programs: Vec<&str> = log.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(
        programs,
        ["port.appimage", "QuiverLauncher-linux-x64.AppImage"]
    );
}

fn collection() -> Manifest {
    Manifest::parse(
        r#"
id = "testcollection"
title = "Test Collection"
[quiver]
name = "TestCollection"
repository = "test/collection"
asset-filter = "windows.zip"
runner = "proton"
executable = "Play.exe"
[original]
platform = "ps4"
title-ids = ["CUSA02320"]
files = ["eboot.bin"]
[[games]]
id = "u2"
title = "Test: Second"
[provide]
kind = "dump"
settings = "test-launcher/settings.json"
game-dir-key = "game_dir"
game-key = "start"
"#,
    )
    .unwrap()
}

#[test]
fn runs_a_windows_port_under_proton_with_the_dump_and_game_chosen() {
    let s = setup(Installs::Always);
    fs::create_dir_all(s.quiver_dir.join("release")).unwrap();
    fs::write(s.quiver_dir.join("release/Play.exe"), b"MZ").unwrap();
    fs::write(s.quiver_dir.join("release/Launcher.exe"), b"MZ").unwrap();
    // Quiver's --run would start this script natively instead of any .exe file.
    fs::write(
        s.quiver_dir.join("release/run.py"),
        "#!/usr/bin/env python3\r\n",
    )
    .unwrap();
    let dump = common::ps4_dump(s.tmp.path(), "CUSA02320", &["eboot.bin"]);
    let server = romm();

    launch_with(
        &s,
        &server,
        &collection(),
        &OriginalLocation::Dump(dump.clone()),
        Some("u2"),
        Duration::from_secs(20),
    )
    .unwrap();

    // Quiver installs; heimspiel runs the Manifest's program under Proton itself.
    assert_eq!(calls(&s), ["--download TestCollection"]);
    let app = s.quiver_dir.join("Apps/TestCollection");
    let prefix = s.quiver_dir.join("Prefixes/TestCollection");
    let proton = fs::read_to_string(s.tmp.path().join("proton/calls.log")).unwrap();
    assert_eq!(
        proton.lines().collect::<Vec<_>>(),
        [
            format!("waitforexitandrun {}", app.join("Play.exe").display()),
            "PROTON_USE_XALIA=0".to_owned(),
            format!(
                "STEAM_COMPAT_CLIENT_INSTALL_PATH={}",
                s.tmp.path().join("steam").display()
            ),
            format!("STEAM_COMPAT_DATA_PATH={}", prefix.display()),
            "SteamGameId=heimspiel-TestCollection".to_owned(),
        ]
    );
    let apps: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(s.quiver_dir.join("apps.json")).unwrap()).unwrap();
    let entry = &apps["apps"][0];
    assert_eq!(entry["linuxRunner"], "proton");
    assert_eq!(entry["linuxPrefixPath"], prefix.to_string_lossy().as_ref());
    let settings = fs::read_to_string(
        prefix.join("pfx/drive_c/users/steamuser/AppData/Roaming/test-launcher/settings.json"),
    )
    .unwrap();
    assert!(settings.contains(r#""start": "u2""#), "{settings}");
    assert!(settings.contains("CUSA02320"), "{settings}");
}

#[test]
fn refuses_a_dump_the_port_does_not_accept() {
    let s = setup(Installs::Always);
    let dump = common::ps4_dump(s.tmp.path(), "CUSA02320", &[]);
    let server = romm();

    let err = launch_with(
        &s,
        &server,
        &collection(),
        &OriginalLocation::Dump(dump),
        Some("u2"),
        Duration::from_secs(20),
    )
    .unwrap_err();

    assert!(format!("{err:#}").contains("eboot.bin"), "{err:#}");
    assert!(calls(&s).is_empty());
}

fn check(s: &Setup, server: &FakeServer, seconds: u64) -> anyhow::Result<()> {
    let manifest = manifest();
    let log = s.tmp.path().join("check.log");
    // `env` stands in for gamescope: it runs the command it is given.
    with_ctx(
        s,
        server,
        &manifest,
        Duration::from_secs(20),
        &["env".to_owned()],
        |ctx| {
            launch::check(
                ctx,
                &manifest.id,
                &OriginalLocation::RomM(7),
                None,
                Duration::from_secs(seconds),
                &log,
            )
        },
    )
}

#[test]
fn checks_that_a_port_keeps_running_and_stops_it() {
    let s = setup(Installs::Always);
    fs::write(s.quiver_dir.join("hangs"), "").unwrap();
    let server = romm();
    let started = Instant::now();

    check(&s, &server, 1).unwrap();

    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(calls(&s), ["--download TestRecomp", "--run TestRecomp"]);
    assert!(
        s.config_home
            .join("TestRecompiled/test.n64.us.1.0.z64")
            .is_file()
    );
}

#[test]
fn fails_the_check_of_a_port_that_ends_at_once() {
    let s = setup(Installs::Always);
    let server = romm();

    let err = check(&s, &server, 5).unwrap_err();

    assert!(format!("{err:#}").contains("ended"), "{err:#}");
}
