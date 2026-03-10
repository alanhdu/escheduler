use std::{
    borrow::Cow,
    collections::VecDeque,
    time::{Duration, Instant},
};

use eyre::Context;
use rand::Rng;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, KeyCode},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::config::{Config, ExerciseIndex};
use crate::db::{Database, Record};

pub(crate) struct Exercise<'id> {
    pub(crate) idx: ExerciseIndex<'id>,
    pub(crate) prev_best: u16,
}

pub(crate) struct App<'a, 'id> {
    pub(crate) config: &'a Config<'id>,
    pub(crate) db: Database,
    pub(crate) input_buffer: String,
    pub(crate) queue: VecDeque<Exercise<'id>>,
    pub(crate) start: Instant,
    pub(crate) session_counter: u32,
}

impl<'a, 'id> App<'a, 'id> {
    pub(crate) fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> eyre::Result<bool> {
        loop {
            terminal.draw(|f| self.render(f))?;
            let next = 1000 - self.start.elapsed().subsec_millis();

            if !(event::poll(Duration::from_millis(next.into()))?) {
                continue;
            }
            while event::poll(Duration::ZERO)?
                && let event::Event::Key(key) = event::read()?
            {
                if !key.is_press() {
                    continue;
                }
                match key.code {
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }
                    KeyCode::Enter => {
                        if self.input_buffer.is_empty() {
                            continue;
                        }
                        let Some(exercise) = self.queue.pop_front() else {
                            return Ok(true);
                        };
                        let reps = self.input_buffer.parse()?;
                        let record = Record {
                            name: self.config.get_name(exercise.idx),
                            weight: self.config.get_weight(exercise.idx),
                            reps,
                        };
                        self.db.write(&record).wrap_err_with(|| {
                            format!("Could not insert {:?}", record)
                        })?;
                        self.input_buffer.clear();
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(self.start.elapsed() >= self.config.duration);
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        self.input_buffer.push(c);
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn render(&self, frame: &mut Frame) {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(2)
            .constraints([Constraint::Min(20), Constraint::Percentage(80)])
            .split(frame.area());

        // 1. Information Display

        let remaining =
            self.config.duration.saturating_sub(self.start.elapsed()).as_secs();
        let rows = [
            Row::new([
                Cell::from("Time Remaining").bold(),
                Cell::from(format!(
                    "{:02}:{:02}",
                    remaining / 60,
                    remaining % 60
                )),
            ])
            .red(),
            Row::new([
                Cell::from("Session").bold(),
                Cell::from(format!("{}", self.session_counter)),
            ]),
            Row::new([
                Cell::from("Target").bold(),
                Cell::from(match self.session_counter % 3 {
                    0 => "High Reps",
                    1 => "Balance",
                    _ => "High Weight",
                }),
            ]),
        ];
        let block = Block::default()
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);
        frame.render_widget(
            Table::new(rows, [Constraint::Length(16), Constraint::Fill(1)])
                .block(block.clone().title("Status")),
            panels[0],
        );

        // 2. Main Area
        let rows = self.queue.iter().enumerate().map(|(i, exercise)| {
            let weight = self.config.get_weight(exercise.idx);
            let name = self.config.get_name(exercise.idx);
            let name = Cell::new(self.config.get_name(exercise.idx).replace(
                "{}",
                &self.config.get_weight(exercise.idx).to_string(),
            ));

            let best = Cell::new(exercise.prev_best.to_string());
            if i > 0 {
                Row::new([name, best, Cell::new("")])
            } else {
                Row::new([
                    name,
                    best,
                    Cell::new(&*self.input_buffer)
                        .style(Style::new().reversed()),
                ])
                .style(Style::new().underlined().italic())
            }
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(35),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .column_spacing(1)
        .header(
            Row::new(["Exercise", "Prev Best", "Count"])
                .style(Style::new().bold()),
        );
        frame.render_widget(table, panels[1]);
    }
}
