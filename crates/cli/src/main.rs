use std::env;
use std::path::PathBuf;
use std::process::{Command as Process, Stdio};
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};
use heimspiel_core::cache::GameCache;
use heimspiel_core::catalog;
use heimspiel_core::config::{Config, RommConfig};
use heimspiel_core::fetch::Fetch;
use heimspiel_core::hook;
use heimspiel_core::launch::{self, OriginalLocation, PortLaunch};
use heimspiel_core::port::{self, Manifest};
use heimspiel_core::progress::{Progress, Window};
use heimspiel_core::proton;
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
    /// List the Ports heimspiel sets up and what each still needs, and other Ports for games you
    /// own.
    Ports {
        /// Also list Ports without a release, which you would have to build yourself.
        #[arg(long)]
        all: bool,
    },
    /// Install and provision a Port, start it unseen in a headless gamescope and check that it is
    /// still running after a while. Needs gamescope.
    Check {
        /// The Port, e.g. `zelda64recomp`.
        port: String,
        /// How long the Port has to keep running.
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
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
        Command::Ports { all } => {
            let mut report = sync::Report::default();
            let (library, dumps) = sync::read_sources(&config, &env, &mut report)?;
            let catalog = catalog::load(
                catalog::URL,
                &env.cache_dir.join("catalog/games.json"),
                Duration::from_secs(24 * 60 * 60),
            )?;
            let quiver = Quiver::new(&env.quiver_dir());
            let manifests = port::builtin();
            let overview = port::overview(&manifests, &library, &dumps, &catalog, &|app| {
                quiver.is_installed(app)
            });
            println!("Ports heimspiel sets up:");
            for state in &overview.manifests {
                let status = match (&state.original, state.installed) {
                    (Some(original), true) => format!("installed; Original: {original}"),
                    (Some(original), false) => {
                        format!("installs on first start; Original: {original}")
                    }
                    (None, _) => format!("needs {}", needed(state.manifest)),
                };
                println!("  {}: {status}", state.manifest.title);
            }
            let (released, to_build): (Vec<_>, Vec<_>) =
                overview.more.iter().partition(|m| m.port.release.is_some());
            let shown = if all {
                overview.more.iter().collect()
            } else {
                released
            };
            if !shown.is_empty() {
                println!(
                    "\nMore Ports for games you own, on GitHub (heimspiel cannot set these up yet):"
                );
                let mut by_game: std::collections::BTreeMap<&str, Vec<_>> = Default::default();
                for m in shown {
                    by_game.entry(m.original.as_str()).or_default().push(m.port);
                }
                for (game, ports) in by_game {
                    println!("  {game}");
                    for port in ports {
                        let release = port.release.as_deref().unwrap_or("build it yourself");
                        println!("    {}  {release}  {}", port.repository, port.kind);
                    }
                }
            }
            if !all && !to_build.is_empty() {
                println!(
                    "\n{} more without a release, to build yourself: heimspiel ports --all",
                    to_build.len()
                );
            }
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
            let progress = progress();
            let result = with_port_launch(&config, &env, &*progress, &[], |ctx| {
                launch::run(ctx, &port, &original, game.as_deref())
            });
            // ES-DE shows nothing when a launch fails.
            if let Err(e) = &result {
                progress.fail(&format!("Could not start {port}: {e:#}"));
            }
            result
        }
        Command::Check { port, seconds } => {
            let manifests = port::builtin();
            let manifest = manifests
                .iter()
                .find(|m| m.id == port)
                .with_context(|| format!("no Port called {port}"))?;
            let mut report = sync::Report::default();
            let (library, dumps) = sync::read_sources(&config, &env, &mut report)?;
            let original = port::original(manifest, &library, &dumps)
                .with_context(|| format!("found no Original for {port}"))?;
            let game = manifest.games.first().map(|g| g.id.as_str());
            let log = env.cache_dir.join("check").join(format!("{port}.log"));
            std::fs::create_dir_all(log.parent().unwrap())?;
            let wrapper: Vec<String> = [
                "gamescope",
                "--backend",
                "headless",
                "-W",
                "1280",
                "-H",
                "720",
                "--",
            ]
            .map(String::from)
            .into();
            with_port_launch(&config, &env, &(), &wrapper, |ctx| {
                launch::check(
                    ctx,
                    &port,
                    &original,
                    game,
                    Duration::from_secs(seconds),
                    &log,
                )
            })
            .with_context(|| format!("{port} failed the check"))?;
            println!(
                "{port} kept running for {seconds} s; output in {}",
                log.display()
            );
            Ok(())
        }
        Command::Hook {
            event: Event::GameStart { args },
        } => {
            let Some(romm) = &config.romm else {
                return Ok(());
            };
            let (client, cache) = romm_access(romm, &env)?;
            let progress = progress();
            let result = hook::game_start(&fetch(&client, &cache, &env, &*progress), &args);
            // Only heimspiel's own games fail here; ES-DE would start the emulator regardless.
            if let Err(e) = &result {
                progress.fail(&format!("{e:#}"));
            }
            result
        }
    }
}

/// Sets up what launching a Port needs and hands it to `f`.
fn with_port_launch(
    config: &Config,
    env: &Env,
    progress: &dyn Progress,
    wrapper: &[String],
    f: impl FnOnce(&PortLaunch) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let romm = match &config.romm {
        Some(romm) => Some(romm_access(romm, env)?),
        None => None,
    };
    let fetch = romm
        .as_ref()
        .map(|(client, cache)| fetch(client, cache, env, progress));
    let quiver = Quiver::new(&env.quiver_dir());
    let base = |var: &str, fallback: &str| {
        env::var_os(var)
            .map(PathBuf::from)
            .unwrap_or_else(|| env.user_home.join(fallback))
    };
    let homes = Homes {
        config: base("XDG_CONFIG_HOME", ".config"),
        data: base("XDG_DATA_HOME", ".local/share"),
    };
    let proton = sync::steam_root(config, &env.user_home).and_then(|root| proton::find(&root));
    let ctx = PortLaunch {
        manifests: &port::builtin(),
        quiver: &quiver,
        proton: proton.as_ref(),
        fetch: fetch.as_ref(),
        progress,
        homes: &homes,
        install_timeout: Duration::from_secs(30 * 60),
        wrapper,
        latest_version: &|app| quiver::latest_tag(&app.repository),
    };
    quiver.ensure_installed()?;
    f(&ctx)
}

/// What a Port's Original is, for someone who does not have it yet.
fn needed(manifest: &Manifest) -> String {
    let original = &manifest.original;
    if !original.title_ids.is_empty() {
        return format!("a Dump of {}", original.title_ids.join(" or "));
    }
    let releases: Vec<String> = original
        .releases
        .iter()
        .map(|r| match &r.revision {
            Some(revision) => format!("{} Rev {revision}", r.region),
            None => r.region.clone(),
        })
        .collect();
    format!(
        "its {} Original in RomM ({})",
        original.platform,
        releases.join(", ")
    )
}

fn romm_access(romm: &RommConfig, env: &Env) -> anyhow::Result<(Romm, GameCache)> {
    let token = sync::read_token(&romm.token_file(&env.user_home, &env.config_dir))?;
    let client = Romm::new(&romm.url, &token);
    let cache = GameCache::new(&env.game_cache(), romm.cache_limit_bytes());
    Ok((client, cache))
}

fn fetch<'a>(
    client: &'a Romm,
    cache: &'a GameCache,
    env: &'a Env,
    progress: &'a dyn Progress,
) -> Fetch<'a> {
    Fetch {
        client,
        cache,
        covers: env.cover_cache(),
        progress,
        show_from_bytes: 64 << 20,
    }
}

/// The loading window of `heimspiel-app`, from next to this binary or from `PATH`; desktop
/// notifications without it.
fn progress() -> Box<dyn Progress> {
    let beside = env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("heimspiel-app")));
    let on_path = env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join("heimspiel-app"));
    match beside.into_iter().chain(on_path).find(|app| app.is_file()) {
        Some(app) => Box::new(Window::new(vec![
            app.to_string_lossy().into_owned(),
            "progress".into(),
        ])),
        None => Box::new(Notifications),
    }
}

struct Notifications;

impl Progress for Notifications {
    fn start(&self, game: &str, _cover: Option<&std::path::Path>, action: &str) {
        notify(&format!("{action} {game} …"));
    }

    fn advance(&self, _done_bytes: u64, _total_bytes: u64) {}

    fn finish(&self) {}

    fn fail(&self, message: &str) {
        notify(message);
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
