use std::env;
use std::path::PathBuf;
use std::process::{Command as Process, Stdio};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use heimspiel_core::cache::GameCache;
use heimspiel_core::config::Config;
use heimspiel_core::hook::{self, GameStart};
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
    /// Launch an Entry; ES-DE calls this with the path of the game file (%ROM%).
    Launch {
        /// Path of the game file or Placeholder, as passed by ES-DE.
        rom: PathBuf,
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
                "Synced {} Steam, {} Heroic, {} manual, {} RomM and {} local games into ES-DE.",
                report.steam, report.heroic, report.manual, report.romm, report.local
            );
            for note in report.notes {
                println!("{note}");
            }
            Ok(())
        }
        Command::Launch { .. } => bail!("`heimspiel launch` is not implemented yet"),
        Command::Hook {
            event: Event::GameStart { args },
        } => {
            let Some(romm) = &config.romm else {
                return Ok(());
            };
            let token = sync::read_token(&romm.token_file(&user_home, &env.config_dir))?;
            let client = Romm::new(&romm.url, &token);
            let cache = GameCache::new(&env.game_cache(), romm.cache_limit_bytes());
            let ctx = GameStart {
                client: &client,
                cache: &cache,
                notify: &notify,
                notify_from_bytes: 64 << 20,
            };
            hook::game_start(&ctx, &args)
        }
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
