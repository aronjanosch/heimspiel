use std::env;
use std::path::PathBuf;
use std::process::{Command as Process, Stdio};
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};
use heimspiel_core::cache::GameCache;
use heimspiel_core::config::{Config, RommConfig};
use heimspiel_core::fetch::Fetch;
use heimspiel_core::hook;
use heimspiel_core::launch::{self, OriginalLocation, PortLaunch};
use heimspiel_core::port;
use heimspiel_core::provision::Homes;
use heimspiel_core::quiver::{self, Quiver};
use heimspiel_core::romm::Romm;
use heimspiel_core::sync::{self, Env};

/// Brings your whole game collection under one roof and presents it in ES-DE.
#[derive(Parser)]
#[command(name = "heimspiel", version)]
struct Cli {
    /// Configuration file [default: ~/.config/heimspiel/config.toml]
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read all Sources and write the Library into ES-DE. Run it while ES-DE is closed.
    Sync,
    /// Install, provision and run a Port; the entries `heimspiel sync` writes call this.
    #[command(group = clap::ArgGroup::new("original").required(true))]
    Launch {
        /// The Port, e.g. `zelda64recomp`.
        port: String,
        /// RomM id of the Original.
        #[arg(long, group = "original")]
        romm: Option<u64>,
        /// Folder of the Dump that is the Original.
        #[arg(long, group = "original")]
        dump: Option<PathBuf>,
        /// The game to start, for Ports with several, e.g. `u1`.
        #[arg(long)]
        game: Option<String>,
    },
    /// Handle an ES-DE event; called by the scripts `heimspiel sync` installs.
    #[command(hide = true)]
    Hook {
        #[command(subcommand)]
        event: Event,
    },
}

#[derive(Subcommand)]
enum Event {
    /// Fetch a RomM game before the emulator opens it.
    GameStart {
        /// The arguments ES-DE passes: game path, name, system, system name.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let user_home = env::home_dir().context("no home directory")?;
    let xdg = |var: &str, fallback: &str| {
        env::var_os(var)
            .map(PathBuf::from)
            .unwrap_or_else(|| user_home.join(fallback))
            .join("heimspiel")
    };
    // The ES-DE hook runs this same binary, with the same configuration file.
    let mut heimspiel = vec![env::current_exe()?.to_string_lossy().into_owned()];
    if let Some(config) = &cli.config {
        heimspiel.extend([
            "--config".to_owned(),
            std::path::absolute(config)?.to_string_lossy().into_owned(),
        ]);
    }
    let env = Env {
        config_dir: xdg("XDG_CONFIG_HOME", ".config"),
        cache_dir: xdg("XDG_CACHE_HOME", ".cache"),
        data_dir: xdg("XDG_DATA_HOME", ".local/share"),
        heimspiel,
        user_home: user_home.clone(),
    };
    let config_path = cli
        .config
        .unwrap_or_else(|| env.config_dir.join("config.toml"));
    let config = Config::load(&config_path)?;
    match cli.command {
        Command::Sync => {
            let report = sync::sync(&config, &env)?;
            println!(
                "Synced {} Steam, {} Heroic, {} manual, {} RomM, {} local games and {} Ports into ES-DE.",
                report.steam, report.heroic, report.manual, report.romm, report.local, report.ports
            );
            for note in report.notes {
                println!("{note}");
            }
            Ok(())
        }
        Command::Launch {
            port,
            romm: rom_id,
            dump,
            game,
        } => {
            let original = match (rom_id, dump) {
                (Some(rom_id), _) => OriginalLocation::RomM(rom_id),
                (None, Some(dump)) => OriginalLocation::Dump(std::path::absolute(dump)?),
                (None, None) => unreachable!("clap requires one"),
            };
            let romm = match &config.romm {
                Some(romm) => Some(romm_access(romm, &env)?),
                None => None,
            };
            let fetch = romm.as_ref().map(|(client, cache)| fetch(client, cache));
            let quiver = Quiver::new(&env.quiver_dir());
            let base = |var: &str, fallback: &str| {
                env::var_os(var)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| user_home.join(fallback))
            };
            let homes = Homes {
                config: base("XDG_CONFIG_HOME", ".config"),
                data: base("XDG_DATA_HOME", ".local/share"),
            };
            let ctx = PortLaunch {
                manifests: &port::builtin(),
                quiver: &quiver,
                fetch: fetch.as_ref(),
                notify: &notify,
                homes: &homes,
                install_timeout: Duration::from_secs(30 * 60),
                latest_version: &|app| quiver::latest_tag(&app.repository),
            };
            let result = quiver
                .ensure_installed()
                .and_then(|_| launch::run(&ctx, &port, &original, game.as_deref()));
            // ES-DE shows nothing when a launch fails.
            if let Err(e) = &result {
                notify(&format!("Could not start {port}: {e:#}"));
            }
            result
        }
        Command::Hook {
            event: Event::GameStart { args },
        } => {
            let Some(romm) = &config.romm else {
                return Ok(());
            };
            let (client, cache) = romm_access(romm, &env)?;
            hook::game_start(&fetch(&client, &cache), &args)
        }
    }
}

fn romm_access(romm: &RommConfig, env: &Env) -> anyhow::Result<(Romm, GameCache)> {
    let token = sync::read_token(&romm.token_file(&env.user_home, &env.config_dir))?;
    let client = Romm::new(&romm.url, &token);
    let cache = GameCache::new(&env.game_cache(), romm.cache_limit_bytes());
    Ok((client, cache))
}

fn fetch<'a>(client: &'a Romm, cache: &'a GameCache) -> Fetch<'a> {
    Fetch {
        client,
        cache,
        notify: &notify,
        notify_from_bytes: 64 << 20,
    }
}

/// A desktop notification; works on Hyprland, KDE and GNOME alike. Silently skipped without
/// `notify-send`.
fn notify(text: &str) {
    let _ = Process::new("notify-send")
        .args(["--app-name=heimspiel", "heimspiel", text])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
