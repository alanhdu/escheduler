#![allow(dead_code)] // still building scaffolding
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    config: PathBuf,

    #[arg(short, long)]
    length: u8,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    dbg!(cli);
    Ok(())
}
