#![allow(dead_code)] // still building scaffolding
mod buffer;
mod config;

use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;

#[derive(Parser, Debug)]
struct Cli {
    config: PathBuf,

    #[arg(short, long)]
    length: u8,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config = Config::from_file(cli.config.as_ref())?;

    dbg!(&config);

    Ok(())
}
