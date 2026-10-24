use std::env;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use heimspiel_core::config::Config;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let user_home = env::home_dir().context("no home directory")?;
    let config_path = cli.config.unwrap_or_else(|| {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| user_home.join(".config"))
            .join("heimspiel/config.toml")
    });
    let config = Config::load(&config_path)?;
    match cli.command {
        Command::Sync => {
            let report = heimspiel_core::sync::sync(&config, &user_home)?;
            println!(
                "Synced {} Steam, {} Heroic and {} manual games into ES-DE.",
                report.steam, report.heroic, report.manual
            );
            for note in report.notes {
                println!("{note}");
            }
            Ok(())
        }
        Command::Launch { .. } => bail!("`heimspiel launch` is not implemented yet"),
    }
}
