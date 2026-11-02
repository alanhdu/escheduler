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
use rand::{Rng, RngExt};

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
    let kind = session.balanced_select([
        SessionKind::Heavy,
        SessionKind::Light,
        SessionKind::Light,
    ]);

    Lifetime::with_lifetime(|lifetime| {
        let mut interner = StringInterner::with_lifetime(lifetime);

        let mut lower = ["hinge", "squat"];
        session.balanced_shuffle(&mut lower);


        let mut pull = ["hpull", "vpull"];
        session.balanced_shuffle(&mut pull);

        let mut push = ["push up", "dip", "overhead"];
        session.balanced_shuffle(&mut push);

        let upper = session.balanced_select(
            [
                [pull[0], push[0], pull[1]],
                [push[0], pull[0], push[1]],
            ]
        );
        let core = session.balanced_select(["flexion", "anti-extension", "rotation"]);
        let aux = session.balanced_select(["knee", "lateral", "lateral"]);

        let q = [
            "power",  // +2 = 2
            lower[0], // +3 = 5
            upper[0], // +3 = 8
            lower[1], // +3 = 11
            upper[1], // +3 = 14
            upper[2], // +2 = 16
            aux,      // +2 = 18
            core,     // +2 = 20
            "carry",  // +2 = 22
        ]
        .map(|x| interner.insert(x));

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

        let queue = q
            .into_iter()
            .map(|x| get_exercise(x))
            .collect::<Result<VecDeque<_>, _>>()?;
        let mut app = App {
            config: &config,
            db,
            input_buffer: String::with_capacity(3),
            queue,
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
