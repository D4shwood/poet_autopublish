pub use poetautopublish::{
    authors, compose, config, history, lang, poem, prosody, search, shared, stats, variants, volume,
};

mod build;
mod install;
mod latex;
mod open;
mod sync;
mod tui;
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
    /// Sync the poems folder with its git remote now, and push the PDFs.
    Sync,
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
        Some(Cmd::Sync) => {
            let cfg = config::Config::load()?;
            if !cfg.poems_dir().join(".git").exists() {
                anyhow::bail!("{} is not a git repo", cfg.poems_dir().display());
            }
            let out = sync::sync(&cfg.poems_dir())?;
            for c in &out.conflicts {
                println!("both changed: {c} (yours kept as \"{c} ({})\")", sync::SIDE);
            }
            println!(
                "{}{}",
                if out.pulled { "pulled · " } else { "" },
                if out.pushed { "pushed" } else { "up to date" }
            );
            if out.pulled {
                build::build(&cfg, None, false)?;
            }
            if sync::push_pdfs(&cfg)? {
                println!("PDFs pushed to the pdf branch");
            }
            Ok(())
        }
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
