mod command;

use crate::prelude::*;
use anyhow::Result;
use std::{io, thread, time::Duration};
use termion::{
  event::Key,
  input::{MouseTerminal, TermRead},
  raw::IntoRawMode,
  screen::AlternateScreen,
};
use tui::{
  backend::TermionBackend,
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  text::{Span, Spans, Text},
  widgets::{Block, Borders, List, ListItem, Paragraph},
  Terminal as TuiTerminal,
};

enum Event<I> {
  Input(I),
  Tick,
}

pub fn log(msg: impl AsRef<str>) {
  let msg = msg.as_ref();
  // todo: actually log
}

pub struct Terminal {
  events: Receiver<Event<Key>>,
  input: String,
}

impl Terminal {
  pub fn new() {
    let (tx, events) = bounded(0);

    thread::spawn({
      let tx = tx.clone();
      move || {
        let stdin = io::stdin();
        for evt in stdin.keys() {
          if let Ok(key) = evt {
            let _ = tx.send(Event::Input(key));
          }
        }
      }
    });

    thread::spawn(move || {
      let _ = tx.send(Event::Tick);
      thread::sleep(Duration::from_millis(200));
    });

    let mut t = Terminal {
      events,
      input: String::new(),
    };

    t.render_loop();
  }

  pub fn render_loop(&mut self) -> Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = TuiTerminal::new(backend)?;

    loop {
      terminal.draw(|f| {
        let chunks = Layout::default()
          .direction(Direction::Vertical)
          // .margin(2)
          .constraints(
            [
              Constraint::Length(3),
              Constraint::Length(3),
              Constraint::Min(1),
            ]
            .as_ref(),
          )
          .split(f.size());

        let input = Paragraph::new(self.input.as_ref())
          .style(Style::default())
          .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[0]);
      })?;

      match self.events.recv()? {
        Event::Input(k) => match k {
          Key::Char('Q') => {
            drop(terminal);
            std::process::exit(0);
          }
          Key::Char(ch) => {
            self.input.push(ch);
          }
          Key::Backspace => {
            self.input.pop();
          }
          Key::End => {
            let cmd = std::mem::replace(&mut self.input, String::new());
            command::parse_command(cmd);
          }
          _ => (),
        },
        Event::Tick => (),
      }
    }
  }
}
