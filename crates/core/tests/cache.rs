use std::fs;
use std::io::Write;
use std::thread;
use std::time::Duration;

use heimspiel_core::cache::GameCache;

fn fill(cache: &GameCache, id: u64, size: usize) {
    cache
        .store(id, "game.bin", size as u64, |part| {
            fs::write(part, vec![0u8; size])?;
            Ok(())
        })
        .unwrap();
}

fn cached(cache: &GameCache, id: u64) -> bool {
    cache.path(id, "game.bin").exists()
}

#[test]
fn stores_a_game_under_its_id() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    let path = cache
        .store(7, "Lode Runner 3-D (USA).n64", 3, |part| {
            fs::write(part, b"rom")?;
            Ok(())
        })
        .unwrap();

    assert_eq!(fs::read(&path).unwrap(), b"rom");
    assert_eq!(path, cache.path(7, "Lode Runner 3-D (USA).n64"));
    assert_eq!(cache.rom_id_of(&path), Some(7));
    assert!(cache.mark_played(&path));
    assert!(!cache.mark_played(&cache.path(8, "Other.n64")));
}

#[test]
fn keeps_the_files_of_a_game_in_subfolders_but_never_outside_its_own() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    assert_eq!(
        cache.path(7, "data/Stage/0.pxa"),
        tmp.path().join("7/data/Stage/0.pxa")
    );
    assert_eq!(
        cache.path(7, "../../etc/passwd"),
        tmp.path().join("7/etc/passwd")
    );
}

#[test]
fn makes_room_by_dropping_the_least_recently_played_games() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);
    fill(&cache, 1, 400);
    thread::sleep(Duration::from_millis(20));
    fill(&cache, 2, 400);
    thread::sleep(Duration::from_millis(20));
    // Playing game 1 again makes game 2 the oldest.
    assert!(cache.mark_played(&cache.path(1, "game.bin")));
    thread::sleep(Duration::from_millis(20));

    fill(&cache, 3, 400);

    assert!(cached(&cache, 1));
    assert!(!cached(&cache, 2));
    assert!(cached(&cache, 3));
}

#[test]
fn keeps_a_cut_short_download_to_go_on_with_it() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    let result = cache.store(1, "game.bin", 8, |part| {
        fs::write(part, b"half")?;
        anyhow::bail!("connection lost")
    });
    assert!(result.is_err());
    assert!(!cached(&cache, 1));

    let path = cache
        .store(1, "game.bin", 8, |part| {
            assert_eq!(fs::read(part)?, b"half");
            fs::OpenOptions::new()
                .append(true)
                .open(part)?
                .write_all(b"done")?;
            Ok(())
        })
        .unwrap();

    assert_eq!(fs::read(path).unwrap(), b"halfdone");
}

#[test]
fn never_keeps_a_file_of_the_wrong_size() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    let result = cache.store(1, "game.bin", 8, |part| {
        fs::write(part, b"short")?;
        Ok(())
    });

    assert!(result.is_err());
    assert!(!cached(&cache, 1));
    assert!(!tmp.path().join("1/game.bin.part").exists());
}
