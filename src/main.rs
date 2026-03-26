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
use crate::config::{Config, RawConfig, Session, SessionKind};
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

    let mut session = Session::from_session_number(session_counter);
    let kind =
        session.balanced_select([SessionKind::Heavy, SessionKind::Light, SessionKind::Light]);

    Lifetime::with_lifetime(|lifetime| {
        let mut interner = StringInterner::with_lifetime(lifetime);

        let mut lower = [interner.insert("hinge"), interner.insert("squat")];
        session.balanced_shuffle(&mut lower);
        let [lower1, lower2] = lower;
        let lower3 = session.balanced_select([
            interner.insert("lateral"),
            interner.insert("power"),
        ]);

        let (upper1, upper2, upper3) = session.balanced_select([
            (
                interner.insert("vertical pull"),
                interner.insert("vertical push"),
                interner.insert("shoulder"),
            ),
            (
                interner.insert("dip"),
                interner.insert("horizontal pull"),
                interner.insert("horizontal push"),
            ),
        ]);

        let core1 = session.balanced_select([
            interner.insert("anti-extension"),
            interner.insert("anti-extension"),
            interner.insert("anti-lateral"),
        ]);
        let accessory = session.balanced_select([
            interner.insert("knee flexion"),
            interner.insert("anti-rotation"),
            interner.insert("accessory"),
        ]);

        let config = Config::from_raw(
            interner,
            value,
            Duration::from_mins(cli.minutes.into()),
            kind,
        )?;

        let mut get_exercise = |group| {
            // TODO: should this also be a balanced RNG?
            let idx = config
                .get_exercise(&mut session.rng(), group)
                .ok_or_else(|| {
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

        let queue = vec![
            get_exercise(lower1)?,
            get_exercise(upper1)?,
            get_exercise(lower2)?,
            get_exercise(upper2)?,
            get_exercise(core1)?,
            get_exercise(lower3)?,
            get_exercise(upper3)?,
            get_exercise(accessory)?,
        ];

        let mut app = App {
            config: &config,
            db,
            input_buffer: String::with_capacity(3),
            queue: VecDeque::from(queue),
            start: Instant::now(),
            session: kind,
        };

        let completed = ratatui::run(|terminal| app.run(terminal))?;
        if completed {
            ureq::post(config.get_url()).send_form([("value", "1")])?;
        }
        Ok(())
    })
}
