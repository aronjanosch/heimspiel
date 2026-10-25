//! Provisioning: putting an Original where its Port expects it, in the form it expects, so the
//! Port starts without asking for anything.

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use sha1::{Digest, Sha1};
use xxhash_rust::xxh3::xxh3_64;

use crate::port::Provide;
use crate::process::stop;

/// The user's base folders; tests point them elsewhere.
pub struct Homes {
    /// `~/.config`
    pub config: PathBuf,
    /// `~/.local/share`
    pub data: PathBuf,
}

/// Provisions the Original for `spec`. `port` is the command that runs the installed Port, needed
/// by kinds that let the Port do the work. `fetch_original` is only called when the Port has no
/// valid copy yet.
pub fn provide(
    spec: &Provide,
    homes: &Homes,
    port: Option<Command>,
    fetch_original: impl FnOnce() -> anyhow::Result<PathBuf>,
) -> anyhow::Result<()> {
    match spec {
        Provide::Librecomp {
            config_dir,
            game_id,
            xxh3,
        } => {
            let target = homes.config.join(config_dir).join(format!("{game_id}.z64"));
            if fs::read(&target).is_ok_and(|bytes| xxh3_64(&bytes) == *xxh3) {
                return Ok(());
            }
            let (original, rom) = read_rom(fetch_original)?;
            // librecomp deletes a stored ROM with the wrong hash, so only a verified copy goes in.
            let actual = xxh3_64(&rom);
            if actual != *xxh3 {
                bail!(
                    "{} does not match the release this Port needs (XXH3 {actual:016x}, expected {xxh3:016x})",
                    original.display()
                );
            }
            write_atomically(&target, &rom)
        }
        Provide::O2r { archive, sha1 } => {
            let archive = homes.data.join(archive);
            if archive.is_file() {
                return Ok(());
            }
            let (original, rom) = read_rom(fetch_original)?;
            let actual = format!("{:x}", Sha1::digest(&rom));
            if !sha1.iter().any(|s| s.eq_ignore_ascii_case(&actual)) {
                bail!(
                    "{} does not match a release this Port supports (SHA-1 {actual})",
                    original.display()
                );
            }
            let port = port.context("the Port is not installed")?;
            let dir = archive.parent().unwrap();
            let rom_file = dir.join("heimspiel-original.z64");
            write_atomically(&rom_file, &rom)?;
            let result = extract(port, &rom_file, &archive);
            let _ = fs::remove_file(&rom_file);
            result
        }
        Provide::Dump { .. } => bail!("this Port needs a Dump, not a ROM"),
    }
}

/// Provisions a Dump for a Port of kind `dump`: writes the Dump's folder and the chosen `game`
/// into the launcher's settings in the Port's Proton prefix (`prefix`, Proton's compat data
/// folder), keeping the user's other settings. Proton fills a prefix it creates around files that
/// are already there, so this also works before the Port's first start.
pub fn provide_dump(
    spec: &Provide,
    prefix: &Path,
    dump: &Path,
    game: Option<&str>,
) -> anyhow::Result<()> {
    let Provide::Dump {
        settings,
        game_dir_key,
        game_key,
        set,
    } = spec
    else {
        bail!("this Port needs a ROM, not a Dump");
    };
    let file = prefix
        .join("pfx/drive_c/users/steamuser/AppData/Roaming")
        .join(settings);
    let mut doc: serde_json::Map<String, serde_json::Value> = match fs::read_to_string(&file) {
        Ok(text) => {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", file.display()))?
        }
        Err(_) => Default::default(),
    };
    // Proton maps the Linux root to drive Z:.
    let windows_path = format!("Z:{}", dump.to_string_lossy().replace('/', "\\"));
    doc.insert(game_dir_key.clone(), windows_path.into());
    if let (Some(key), Some(game)) = (game_key, game) {
        doc.insert(key.clone(), game.into());
    }
    doc.extend(set.clone());
    write_atomically(
        &file,
        (serde_json::to_string_pretty(&doc)? + "\n").as_bytes(),
    )
}

/// Fetches the Original and brings it into big-endian order, padded to whole words.
fn read_rom(
    fetch_original: impl FnOnce() -> anyhow::Result<PathBuf>,
) -> anyhow::Result<(PathBuf, Vec<u8>)> {
    let original = fetch_original()?;
    let mut rom = fs::read(&original).with_context(|| format!("reading {}", original.display()))?;
    to_big_endian(&mut rom).with_context(|| format!("reading {}", original.display()))?;
    rom.resize(rom.len().next_multiple_of(4), 0);
    Ok((original, rom))
}

/// Runs the Port with the ROM until it has written `archive`, then stops it: it would otherwise
/// wait at its "Run?" question.
fn extract(mut port: Command, rom: &Path, archive: &Path) -> anyhow::Result<()> {
    let log_path = archive.with_file_name("heimspiel-extract.log");
    let log = fs::File::create(&log_path)?;
    let program = port.get_program().to_string_lossy().into_owned();
    let mut child = port
        .arg(rom)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .spawn()
        .with_context(|| format!("starting {program}"))?;
    let started = Instant::now();
    let mut last_size = None;
    let mut stable_since = Instant::now();
    loop {
        let size = fs::metadata(archive).ok().map(|m| m.len());
        if size != last_size {
            last_size = size;
            stable_since = Instant::now();
        }
        let said_done = fs::read_to_string(&log_path).is_ok_and(|l| l.contains("Done!"));
        // SoH logs "Done!" after extracting; the size check covers ports that log differently.
        if size.is_some() && (said_done || stable_since.elapsed() > Duration::from_secs(3)) {
            stop(&mut child);
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            if archive.is_file() {
                return Ok(());
            }
            bail!(
                "{} ended without writing {}; see {}",
                program,
                archive.display(),
                log_path.display()
            );
        }
        if started.elapsed() > Duration::from_secs(600) {
            stop(&mut child);
            bail!(
                "{} did not write {} within 10 minutes",
                program,
                archive.display()
            );
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn write_atomically(target: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let dir = target.parent().unwrap();
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let tmp = target.with_extension("part");
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, target).with_context(|| format!("replacing {}", target.display()))
}

/// Brings an N64 ROM into big-endian (z64) order, detected from its header rather than its name:
/// RomM serves byte-swapped (v64) files named `.n64`.
fn to_big_endian(rom: &mut [u8]) -> anyhow::Result<()> {
    match rom.get(..4) {
        Some([0x80, 0x37, 0x12, 0x40]) => {}
        Some([0x37, 0x80, 0x40, 0x12]) => rom
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .for_each(|pair| pair.swap(0, 1)),
        Some([0x40, 0x12, 0x37, 0x80]) => rom
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .for_each(|word| word.reverse()),
        _ => bail!("not an N64 ROM"),
    }
    Ok(())
}
