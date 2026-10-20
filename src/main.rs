mod authors;
mod build;
mod config;
mod history;
mod install;
mod lang;
mod latex;
mod poem;
mod search;
mod stats;
mod tui;
mod volume;
mod watch;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Write poems, and publish them as weekly PDF volumes.
#[derive(Parser)]
#[command(name = "poetautopublish", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build the PDFs that changed (the default for scripts).
    Build {
        /// Rebuild every PDF.
        #[arg(long)]
        force: bool,
        /// Read poems from this folder instead of the configured one.
        #[arg(long)]
        poems_dir: Option<PathBuf>,
    },
    /// Watch the poems folder and rebuild on every change.
    Watch,
    /// Open the TUI straight into a new poem.
    New,
    /// Install the background service, the `poet` command and the menu entry.
    Install,
    /// Remove what `install` added.
    Uninstall,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => tui::run(false),
        Some(Cmd::New) => tui::run(true),
        Some(Cmd::Build { force, poems_dir }) => {
            let cfg = config::Config::load()?;
            let report = build::build(&cfg, poems_dir.as_deref(), force)?;
            for line in report.summary() {
                println!("{line}");
            }
            if !report.errors.is_empty() {
                std::process::exit(1);
            }
            Ok(())
        }
        Some(Cmd::Watch) => watch::run(),
        Some(Cmd::Install) => {
            config::Config::load()?;
            for l in install::install()? {
                println!("{l}");
            }
            println!("config: {}", config::Config::path().display());
            Ok(())
        }
        Some(Cmd::Uninstall) => {
            for l in install::uninstall()? {
                println!("{l}");
            }
            Ok(())
        }
    }
}
