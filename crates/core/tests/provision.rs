use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use heimspiel_core::port::Provide;
use heimspiel_core::provision::{self, Homes};
use xxhash_rust::xxh3::xxh3_64;

/// A tiny stand-in for an N64 ROM in big-endian (z64) order: header magic, then data.
const Z64: [u8; 12] = [0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4, 5, 6, 7, 8];
/// The same ROM byte-swapped in pairs (v64), as RomM serves many `.n64` files.
const V64: [u8; 12] = [0x37, 0x80, 0x40, 0x12, 2, 1, 4, 3, 6, 5, 8, 7];
/// The same ROM in little-endian words (n64).
const N64: [u8; 12] = [0x40, 0x12, 0x37, 0x80, 4, 3, 2, 1, 8, 7, 6, 5];

fn spec(xxh3: u64) -> Provide {
    Provide::Librecomp {
        config_dir: "TestRecompiled".into(),
        game_id: "test.n64.us.1.0".into(),
        xxh3,
    }
}

fn homes(config: &Path) -> Homes {
    Homes {
        config: config.to_path_buf(),
        data: config.with_file_name("data"),
    }
}

fn rom(dir: &tempfile::TempDir, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join("original.n64");
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn stores_any_byte_order_as_big_endian() {
    for order in [Z64, V64, N64] {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("config");
        let original = rom(&tmp, &order);

        provision::provide(&spec(xxh3_64(&Z64)), &homes(&config), None, || {
            Ok(original.clone())
        })
        .unwrap();

        assert_eq!(
            fs::read(config.join("TestRecompiled/test.n64.us.1.0.z64")).unwrap(),
            Z64
        );
    }
}

#[test]
fn pads_the_rom_to_whole_words() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path().join("config");
    let original = rom(&tmp, &Z64[..10]);
    let padded = [0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4, 5, 6, 0, 0];

    provision::provide(&spec(xxh3_64(&padded)), &homes(&config), None, || {
        Ok(original.clone())
    })
    .unwrap();

    assert_eq!(
        fs::read(config.join("TestRecompiled/test.n64.us.1.0.z64")).unwrap(),
        padded
    );
}

#[test]
fn never_stores_a_rom_with_the_wrong_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path().join("config");
    let original = rom(&tmp, &Z64);

    let err = provision::provide(
        &spec(0x1234),
        &homes(&config),
        None,
        || Ok(original.clone()),
    )
    .unwrap_err();

    assert!(format!("{err:#}").contains("does not match"), "{err:#}");
    assert!(!config.join("TestRecompiled/test.n64.us.1.0.z64").exists());
}

#[test]
fn does_not_fetch_the_original_when_it_is_already_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path().join("config");
    fs::create_dir_all(config.join("TestRecompiled")).unwrap();
    fs::write(config.join("TestRecompiled/test.n64.us.1.0.z64"), Z64).unwrap();
    let fetched = Cell::new(false);

    provision::provide(&spec(xxh3_64(&Z64)), &homes(&config), None, || {
        fetched.set(true);
        anyhow::bail!("must not be called")
    })
    .unwrap();

    assert!(!fetched.get());
}

fn o2r() -> Provide {
    Provide::O2r {
        archive: "soh/oot.o2r".into(),
        sha1: vec![sha1_hex(&Z64)],
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(bytes))
}

/// A stand-in for Ship of Harkinian: extracts the ROM it gets into the archive, then waits for
/// an answer like SoH's "Run SoH?" popup does.
fn fake_port(dir: &Path, archive: &Path) -> PathBuf {
    let exe = dir.join("soh.appimage");
    fs::write(
        &exe,
        format!(
            "#!/bin/sh\nmkdir -p \"$(dirname '{a}')\"\ncp \"$1\" '{a}'\necho 'Done! Took 6190ms'\nexec sleep 600\n",
            a = archive.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    exe
}

#[test]
fn extracts_the_original_once_with_the_port_and_stops_it() {
    let tmp = tempfile::tempdir().unwrap();
    let homes = homes(&tmp.path().join("config"));
    let archive = homes.data.join("soh/oot.o2r");
    let exe = fake_port(tmp.path(), &archive);
    let original = rom(&tmp, &V64);
    let started = Instant::now();

    provision::provide(&o2r(), &homes, Some(&exe), || Ok(original.clone())).unwrap();

    assert!(started.elapsed() < Duration::from_secs(10));
    // The Port got the verified ROM in big-endian order.
    assert_eq!(fs::read(&archive).unwrap(), Z64);
}

#[test]
fn does_not_run_the_port_for_an_unknown_release() {
    let tmp = tempfile::tempdir().unwrap();
    let homes = homes(&tmp.path().join("config"));
    let archive = homes.data.join("soh/oot.o2r");
    let exe = fake_port(tmp.path(), &archive);
    let original = rom(&tmp, &[0x80, 0x37, 0x12, 0x40, 9, 9, 9, 9]);

    let err = provision::provide(&o2r(), &homes, Some(&exe), || Ok(original.clone())).unwrap_err();

    assert!(format!("{err:#}").contains("does not match"), "{err:#}");
    assert!(!archive.exists());
}

#[test]
fn keeps_an_existing_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let homes = homes(&tmp.path().join("config"));
    let archive = homes.data.join("soh/oot.o2r");
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    fs::write(&archive, b"o2r").unwrap();

    provision::provide(&o2r(), &homes, None, || anyhow::bail!("must not be called")).unwrap();

    assert_eq!(fs::read(&archive).unwrap(), b"o2r");
}
