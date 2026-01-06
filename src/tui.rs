use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

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

pub(crate) struct App<'a> {
    pub(crate) config: &'a Config,
    pub(crate) input_buffer: String,
    pub(crate) order: TargetOrder,
    pub(crate) queue: VecDeque<u16>,
    pub(crate) start: Instant,
}

impl<'a> App<'a> {
    pub(crate) fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> eyre::Result<()> {
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
                        self.input_buffer.clear();
                        self.queue.pop_front();
                        self.append_exercise();
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        self.input_buffer.push(c);
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn append_exercise(&mut self) {
        let target = match self.queue.back() {
            None => self.order.first(),
            Some(idx) => self.order.next(self.config.get_target(*idx)),
        };

        let range = self.config.get_target_range(target);
        let mut rng = rand::rng();

        // TODO: use a better sampling strategy?
        loop {
            let idx = rng.random_range(range.clone());
            let candidate = self.config.get_group(idx);
            if self.queue.iter().all(|i| self.config.get_group(*i) != candidate)
            {
                self.queue.push_back(idx);
                break;
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Length(4)])
            .split(panels[0]);

        let remaining = self.config.duration.saturating_sub( self.start.elapsed()).as_secs();
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
            chunks[0],
        );

        frame.render_widget(
            Paragraph::new(format!("{} lbs", self.config.weight))
                .alignment(Alignment::Center)
                .block(block.title("Weight")),
            chunks[1],
        );

        // 2. Main Area
        let rows = self.queue.iter().enumerate().map(|(i, idx)| {
            let exercise = self.config.get_exercise(*idx);

            let target = Cell::new(exercise.target.to_string());
            let name = Cell::new(exercise.name);
            if i > 0 {
                Row::new([target, name, Cell::new("")])
            } else {
                Row::new([
                    target,
                    name,
                    Cell::new(&*self.input_buffer)
                        .style(Style::new().reversed()),
                ])
                .style(Style::new().underlined().italic())
            }
        });

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
            Row::new(["Target", "Exercise", "Count"])
                .style(Style::new().bold()),
        );
        frame.render_widget(table, panels[1]);
    }
}
