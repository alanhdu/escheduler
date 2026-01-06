#![allow(dead_code)] // still building scaffolding
mod buffer;
mod config;
mod tui;

use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use rand::Rng;

use crate::config::{Config, TargetOrder};
use crate::tui::App;

#[derive(Parser, Debug)]
struct Cli {
    config: PathBuf,

    #[arg(short, long)]
    minutes: u8,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let mut rng = rand::rng();

    let config = Config::from_file(
        cli.config.as_ref(),
        Duration::from_mins(cli.minutes.into()),
        5 * rng.random_range(1..=8),
    )?;

    let mut app = App {
        config: &config,
        input_buffer: String::with_capacity(3),
        order: TargetOrder::new(&mut rng),
        queue: VecDeque::with_capacity(9),
        start: Instant::now(),
    };
    for _ in 0..9 {
        app.append_exercise();
    }

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
