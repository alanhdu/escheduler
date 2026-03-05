#![allow(unused)]

mod buffer;
mod config;
mod db;
mod queue;
mod tui;

use std::collections::VecDeque;
use std::fs::File;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use clap::Parser;
use eyre::{Context, eyre};
use rand::Rng;

use crate::buffer::{Lifetime, StringInterner};
use crate::config::{Config, RawConfig};
use crate::db::Database;
use crate::tui::{App, Exercise};

#[derive(Parser, Debug)]
struct Cli {
    config: PathBuf,
    db: PathBuf,

    #[arg(short, long)]
    minutes: u8,
}

fn random_select<T>(rng: &mut impl Rng, a: T, b: T) -> (T, T) {
    if rng.random::<bool>() { (a, b) } else { (b, a) }
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let mut rng = rand::rng();
    let db = Database::open(&cli.db).wrap_err_with(|| {
        format!("Could not open database file `{}`", cli.db.display())
    })?;
    let session_counter = db.session().wrap_err_with(|| {
        format!("Could not query database file `{}`", cli.db.display())
    })?;
    let path: &Path = cli.config.as_ref();

    let mut file = File::open(path).wrap_err_with(|| {
        format!("Config file `{}` could not be read", path.display())
    })?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let value: RawConfig = serde_json::from_str(&buf).wrap_err_with(|| {
        format!("Could not parse file `{}`", path.display())
    })?;

    Lifetime::with_lifetime(|lifetime| {
        let mut interner = StringInterner::with_lifetime(lifetime);

        let (lower1, lower2) = random_select(
            &mut rng,
            interner.insert("hamstring"),
            interner.insert("squat"),
        );

        let pull = if session_counter % 2 == 0 {
            "vertical pull"
        } else {
            "horizontal push"
        };
        let push = if session_counter % 3 == 0 {
            "upward push"
        } else if session_counter % 3 == 1 {
            "downward push"
        } else {
            "horizontal push"
        };
        let (upper1, upper2) = random_select(
            &mut rng,
            interner.insert(push),
            interner.insert(pull),
        );

        let core1 = interner.insert("anti-extension");
        let (core2, _) = random_select(
            &mut rng,
            interner.insert("anti-lateral"),
            interner.insert("anti-rotation"),
        );
        let accessory1 = interner.insert("accessory1");
        let accessory2 = interner.insert("accessory2");
        let shoulder = interner.insert("exterior shoulder");

        let config = Config::from_raw(
            interner,
            value,
            Duration::from_mins(cli.minutes.into()),
            session_counter,
        )?;

        let mut get_exercise = |group| {
            let idx =
                config.get_exercise(&mut rng, group).ok_or_else(|| {
                    eyre!(format!(
                        "Could not find exercise group '{}'",
                        config.get_group(group)
                    ))
                })?;
            Ok::<Exercise<'_>, eyre::ErrReport>(Exercise {
                idx,
                prev_best: db
                    .best(config.get_name(idx), config.get_weight(idx))?,
            })
        };

        let acc1 = get_exercise(accessory1)?;
        let mut acc2 = get_exercise(accessory2)?;
        while (acc1.idx == acc2.idx) {
            acc2 = get_exercise(accessory2)?;
        }

        let queue = vec![
            get_exercise(lower1)?,
            get_exercise(upper1)?,
            get_exercise(lower2)?,
            get_exercise(upper2)?,
            acc1,
            get_exercise(core1)?,
            acc2,
            get_exercise(shoulder)?,
            get_exercise(core2)?,
        ];

        let mut app = App {
            config: &config,
            db,
            input_buffer: String::with_capacity(3),
            queue: VecDeque::from(queue),
            start: Instant::now(),
        };

        let completed = ratatui::run(|terminal| app.run(terminal))?;
        if completed {
            ureq::post(config.get_url()).send_form([("value", "1")])?;
        }
        Ok(())
    })
}
