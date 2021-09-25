mod command;

use crate::prelude::*;
use anyhow::Result;
use std::{collections::VecDeque, io, thread, time::Duration};
use termion::{
  event::Key,
  input::{MouseTerminal, TermRead},
  raw::IntoRawMode,
  screen::AlternateScreen,
};
use tui::{
  backend::TermionBackend,
  layout::{Constraint, Direction, Layout},
  style::Style,
  widgets::{Block, Borders, List, ListItem, Paragraph},
  Terminal as TuiTerminal,
};

enum Event<I> {
  Input(I),
  Tick,
}

lazy_static! {
  pub static ref LOG: (Sender<String>, Receiver<String>) = unbounded();
}

pub fn log(msg: impl AsRef<str>) {
  let _ = LOG.0.send(msg.as_ref().to_string());
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

    thread::spawn(move || loop {
      let _ = tx.send(Event::Tick);
      thread::sleep(Duration::from_millis(200));
    });

    let mut t = Terminal {
      events,
      input: String::new(),
    };

    if let Err(e) = t.render_loop() {
      log!("{:?}", e);
    };
  }

  pub fn render_loop(&mut self) -> Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = TuiTerminal::new(backend)?;

    let mut logs: VecDeque<String> = VecDeque::new();

    loop {
      terminal.draw(|f| {
        let chunks = Layout::default()
          .direction(Direction::Vertical)
          // .margin(2)
          .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
          .split(f.size());

        let input = Paragraph::new(self.input.as_ref())
          .style(Style::default())
          .block(Block::default().borders(Borders::ALL).title("Input"));
        f.set_cursor(
          chunks[0].x + self.input.len() as u16 + 1,
          chunks[0].y + 1,
        );
        f.render_widget(input, chunks[0]);

        let logs: Vec<ListItem> = logs
          .iter()
          .enumerate()
          .rev()
          .take(chunks[1].height as usize)
          .map(|(i, l)| ListItem::new(format!("{}|{}", i, l)))
          .collect();
        f.render_widget(
          List::new(logs)
            .block(Block::default().borders(Borders::TOP).title("Logs")),
          chunks[1],
        );
      })?;

      for log in LOG.1.try_iter() {
        logs.push_back(log);
        if logs.len() > 300 {
          logs.pop_front();
        }
      }

      match self.events.recv()? {
        Event::Input(k) => match k {
          Key::Char('Q') => {
            drop(terminal);
            std::process::exit(0);
          }
          Key::Char('\n') => {
            let cmd = std::mem::replace(&mut self.input, String::new());
            // TODO: isolate into thread
            thread::spawn(move || {
              if let Err(e) = command::parse_command(cmd) {
                log!("Command: {:?}", e);
              }
            });
          }
          Key::Backspace => {
            self.input.pop();
          }
          Key::Char(ch) => {
            self.input.push(ch);
          }
          _ => (),
        },
        Event::Tick => {}
      }
    }
  }
}
