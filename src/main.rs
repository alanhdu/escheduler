#![allow(dead_code)] // still building scaffolding
mod config;

use std::path::PathBuf;

use clap::Parser;
use eyre::Context;

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
    let config = Config::try_from(cli.config.as_ref()).wrap_err_with(|| {
        format!("Config file `{}` could not be read", cli.config.display())
    })?;

    dbg!(&config);

    Ok(())
}
