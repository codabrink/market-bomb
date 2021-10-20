mod command;

use crate::database;
use crate::prelude::*;
use anyhow::Result;
use regex::Regex;
use std::{collections::VecDeque, io, thread, time::Duration};
use termion::{
  event::Key,
  input::{MouseTerminal, TermRead},
  raw::IntoRawMode,
  screen::AlternateScreen,
};
use tui::style::Modifier;
use tui::{
  backend::TermionBackend,
  layout::{Constraint, Direction, Layout},
  style::{Color, Style},
  text::{Span, Spans},
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

lazy_static! {
  static ref RE_FORMAT: Regex =
    Regex::new(r"^(/(?P<fmt>[a-zA-Z]*)\s)?(?P<text>.+)$").unwrap();
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
    let mut cmd_index = 0;
    let mut log_offset = 0;

    loop {
      terminal.draw(|f| {
        let chunks = Layout::default()
          .direction(Direction::Vertical)
          // .margin(2)
          .constraints(
            [
              Constraint::Length(1),
              Constraint::Length(3),
              Constraint::Min(1),
            ]
            .as_ref(),
          )
          .split(f.size());

        let unique_violations = database::UNIQUE_VIOLATIONS.load(Relaxed);
        let candle_count = database::CANDLES.load(Relaxed);
        let stats = Spans::from(vec![
          Span::raw(" uniq err: "),
          Span::styled(
            unique_violations.to_string(),
            Style::default().fg(Color::Yellow),
          ),
          Span::raw(" candles: "),
          Span::styled(
            candle_count.to_string(),
            Style::default().fg(Color::Green),
          ),
        ]);
        f.render_widget(Paragraph::new(stats), chunks[0]);

        let input = Paragraph::new(self.input.as_ref())
          .style(Style::default())
          .block(Block::default().borders(Borders::ALL).title("Input"));
        f.set_cursor(
          chunks[1].x + self.input.len() as u16 + 1,
          chunks[1].y + 1,
        );
        f.render_widget(input, chunks[1]);

        let mut logs: VecDeque<ListItem> = logs
          .iter()
          .rev()
          .enumerate()
          .rev()
          .skip(log_offset)
          .take(chunks[2].height as usize)
          .fold(VecDeque::new(), |mut vec, (i, l)| {
            let i = i.to_string();
            let caps = RE_FORMAT.captures(l).unwrap();
            let fmt = caps.name("fmt").map_or("", |m| m.as_str());
            let text =
              caps.name("text").map_or("MISSING MESSAGE", |m| m.as_str());

            // inneficient
            let new_items: Vec<ListItem> = text
              .chars()
              .collect::<Vec<char>>()
              .chunks(chunks[2].width as usize - (i.len() + 2))
              .map(|c| c.iter().collect::<String>())
              .collect::<Vec<String>>()
              .into_iter()
              .map(|l| {
                let mut style = Style::default();
                for fmt in fmt.chars() {
                  match fmt {
                    'g' => style = style.fg(Color::Green),
                    'b' => style = style.fg(Color::Blue),
                    'y' => style = style.fg(Color::Yellow),
                    'r' => style = style.fg(Color::Red),
                    'B' => style = style.add_modifier(Modifier::BOLD),
                    _ => {}
                  }
                }
                let spans = Spans::from(vec![
                  Span::raw(format!("{}| ", i)),
                  Span::styled(l, style),
                ]);
                ListItem::new(spans)
              })
              .collect();
            vec.extend(new_items);
            vec
          });
        if log_offset > 0 {
          logs.push_front(ListItem::new(format!(
            "--- More ({}) ---",
            log_offset
          )));
        }
        logs.truncate(chunks[2].height as usize);

        f.render_widget(
          List::new(logs)
            .block(Block::default().borders(Borders::TOP).title("Logs")),
          chunks[2],
        );
      })?;

      for log in LOG.1.try_iter() {
        logs.push_front(log);
        logs.truncate(300);
      }

      match self.events.recv()? {
        Event::Input(k) => match k {
          Key::Char('Q') => {
            drop(terminal);
            std::process::exit(0);
          }
          Key::Ctrl('p') => {
            let meta = Meta::load()?;
            let cmd = meta.cmds.get(cmd_index);
            if let Some(cmd) = cmd {
              self.input = cmd.clone();
              cmd_index += 1;
            }
          }
          Key::Ctrl('n') => {
            cmd_index = cmd_index.saturating_sub(1);
            let meta = Meta::load()?;
            let cmd = meta.cmds.get(cmd_index);
            if let Some(cmd) = cmd {
              self.input = cmd.clone();
            }
          }
          // up
          Key::Ctrl('u') => {
            log_offset = log_offset.saturating_sub(1);
          }
          // down
          Key::Ctrl('d') => {
            log_offset = log_offset.saturating_add(1).min(logs.len());
          }
          Key::Char('\n') => {
            let cmd = std::mem::replace(&mut self.input, String::new());
            Meta::log_command(&cmd)?;
            cmd_index = 0;

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
