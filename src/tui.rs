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
    style::Style,
    text::Text,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::config::{Config, TargetOrder};
use crate::db::{Database, Record};

pub(crate) struct Spec {
    idx: u16,
    best: u16,
    weight: u8,
}

pub(crate) struct App<'a> {
    pub(crate) config: &'a Config,
    pub(crate) db: Database,
    pub(crate) input_buffer: String,
    pub(crate) order: TargetOrder,
    pub(crate) queue: VecDeque<Spec>,
    pub(crate) start: Instant,
}

impl<'a> App<'a> {
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
                        let Spec { idx, weight, .. } =
                            self.queue.pop_front().unwrap();
                        let reps = self.input_buffer.parse()?;
                        let record = Record {
                            name: self.config.get_name(idx),
                            weight,
                            reps,
                        };
                        self.db.write(&record).wrap_err_with(|| {
                            format!("Could not insert {:?}", record)
                        })?;

                        self.input_buffer.clear();
                        self.append_exercise()?;
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

    pub(crate) fn append_exercise(&mut self) -> eyre::Result<()> {
        let target = match self.queue.back() {
            None => self.order.first(),
            Some(Spec { idx, .. }) => {
                self.order.next(self.config.get_target(*idx))
            }
        };

        let range = self.config.get_target_range(target);
        let mut rng = rand::rng();

        // TODO: use a better strategy than rejection sampling?
        loop {
            let idx = rng.random_range(range.clone());
            let candidate = self.config.get_group(idx);
            if self
                .queue
                .iter()
                .all(|p| self.config.get_group(p.idx) != candidate)
            {
                let name = self.config.get_name(idx);
                let weight = self.config.get_weight(idx, &mut rng);

                let best = self.db.best(name, weight).wrap_err_with(|| {
                    format!(
                        "Coud not query SQL for name={} weight={}",
                        name, weight
                    )
                })?;
                self.queue.push_back(Spec { idx, best, weight });
                break Ok(());
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
        let text = Text::raw(format!(
            "Time Remaining:\n{:02}:{:02}",
            remaining / 60,
            remaining % 60,
        ))
        .style(Style::new().bold().red());

        let block = Block::default()
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);

        frame.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(block.clone().title("Clock")),
            panels[0],
        );

        // 2. Main Area
        let rows = self.queue.iter().enumerate().map(
            |(i, Spec { idx, best, weight })| {
                let target =
                    Cell::new(self.config.get_target(*idx).to_string());

                let name = Cell::new(if *weight > 0 {
                    Cow::from(format!(
                        "{} ({} lbs)",
                        self.config.get_name(*idx),
                        *weight
                    ))
                } else {
                    Cow::from(self.config.get_name(*idx))
                });
                let best = Cell::new(best.to_string());
                if i > 0 {
                    Row::new([target, name, best, Cell::new("")])
                } else {
                    Row::new([
                        target,
                        name,
                        best,
                        Cell::new(&*self.input_buffer)
                            .style(Style::new().reversed()),
                    ])
                    .style(Style::new().underlined().italic())
                }
            },
        );

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(35),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .column_spacing(1)
        .header(
            Row::new(["Target", "Exercise", "Prev Best", "Count"])
                .style(Style::new().bold()),
        );
        frame.render_widget(table, panels[1]);
    }
}
