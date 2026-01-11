#![allow(dead_code)] // still building scaffolding
mod buffer;
mod config;
mod db;
mod tui;

use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use eyre::Context;

use crate::config::{Config, TargetOrder};
use crate::db::Database;
use crate::tui::App;

#[derive(Parser, Debug)]
struct Cli {
    config: PathBuf,
    db: PathBuf,

    #[arg(short, long)]
    minutes: u8,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let mut rng = rand::rng();

    let db = Database::open(&cli.db).wrap_err_with(|| {
        format!("Could not open database file `{}`", cli.db.display())
    })?;

    let config = Config::from_file(
        cli.config.as_ref(),
        Duration::from_mins(cli.minutes.into()),
    )?;

    let mut app = App {
        config: &config,
        db,
        input_buffer: String::with_capacity(3),
        order: TargetOrder::new(&mut rng),
        queue: VecDeque::with_capacity(9),
        start: Instant::now(),
    };
    for _ in 0..9 {
        app.append_exercise()?;
    }

    let completed = ratatui::run(|terminal| app.run(terminal))?;
    if completed {
        ureq::post(config.get_url()).send_form([("value", "1")])?;
    }
    Ok(())
}
