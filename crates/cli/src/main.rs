use std::path::PathBuf;

use anyhow::bail;
use clap::{Parser, Subcommand};

/// Brings your whole game collection under one roof and presents it in ES-DE.
#[derive(Parser)]
#[command(name = "heimspiel", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read all Sources and write the Library into the Outputs.
    Sync,
    /// Launch an Entry; ES-DE calls this with the path of the game file (%ROM%).
    Launch {
        /// Path of the game file or Placeholder, as passed by ES-DE.
        rom: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Sync => bail!("`heimspiel sync` is not implemented yet"),
        Command::Launch { .. } => bail!("`heimspiel launch` is not implemented yet"),
    }
}
