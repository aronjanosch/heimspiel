use std::fs;
use std::thread;
use std::time::Duration;

use heimspiel_core::cache::GameCache;

fn fill(cache: &GameCache, id: u64, size: usize) {
    cache
        .store(id, "game.bin", size as u64, |file| {
            fs::write(file, vec![0u8; size])?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn stores_a_game_under_its_id_and_reuses_it() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    let path = cache
        .store(7, "Lode Runner 3-D (USA).n64", 3, |file| {
            fs::write(file, b"rom")?;
            Ok(())
        })
        .unwrap();

    assert_eq!(fs::read(&path).unwrap(), b"rom");
    assert_eq!(cache.get(7, "Lode Runner 3-D (USA).n64"), Some(path));
    assert_eq!(cache.get(8, "Other.n64"), None);
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
    cache.get(1, "game.bin").unwrap();
    thread::sleep(Duration::from_millis(20));

    fill(&cache, 3, 400);

    assert!(cache.get(1, "game.bin").is_some());
    assert!(cache.get(2, "game.bin").is_none());
    assert!(cache.get(3, "game.bin").is_some());
}

#[test]
fn keeps_nothing_half_downloaded() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = GameCache::new(tmp.path(), 1000);

    let result = cache.store(1, "game.bin", 10, |file| {
        fs::write(file, b"half")?;
        anyhow::bail!("connection lost")
    });

    assert!(result.is_err());
    assert!(cache.get(1, "game.bin").is_none());
}
