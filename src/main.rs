#[macro_use]
extern crate lazy_static;
extern crate env_logger;
extern crate r2d2;

mod prelude;

mod api;
mod config;
mod core;
pub mod database;
mod frame;
mod terminal;
mod web_server;

use anyhow::Result;
use indicatif::ProgressBar;
use prelude::*;
use std::{thread, time::Duration};

fn main() {
  Config::load();
  database::candle_counting_thread();
  terminal::Terminal::new();
}

fn build_history() {
  let config = Config::load();

  for step in ["1d"] {
    let mut q = Query::default();
    q.set_all(&[
      Start(format!("{}d", config.history_start).ago()),
      End(format!("{}d", config.history_end).ago()),
    ]);

    q.set_interval(step);
    API.fetch_candles(&mut q);
  }
}

fn train(symbol: &str, interval: &str) -> Result<()> {
  // Collect all candles
  let con = con();

  let folder_path = format!(
    "builder/csv/train/{symbol}/{interval}/{candles_forward}",
    symbol = symbol,
    interval = interval,
    candles_forward = CONFIG.predict_candles_forward()
  );
  let _ = fs::remove_dir_all(&folder_path);
  let _ = fs::remove_file("builder/features.npy");
  let _ = fs::remove_file("builder/labels.npy");

  let step = interval.ms();
  let now = now().round(step);
  let then = now - CONFIG.history_num_candles * step;

  log!("Writing frames...");
  let pb = ProgressBar::new(((now - then) / step) as u64);
  let mut ms = then;
  // while ms < now {
  // match Frame::new(&mut con, symbol, interval, ms) {
  // Ok(frame) => {
  // let _ = frame.write_to_csv(&folder_path);
  // }
  // Err(e) => {
  // log!("{:?}", e);
  // }
  // }
  // pb.inc(1);
  // ms += step;
  // }
  pb.finish();
  Ok(())
}

fn predict_now(symbol: &str, interval: &str) -> Result<()> {
  let mut con = con();
  let _ = fs::remove_dir_all("builder/csv/predict");

  let step = interval.ms();
  let now = now().round(step);
  // match Frame::new(&mut con, symbol, interval, now) {
  // Ok(frame) => {
  // frame.predict(CONFIG.predict_candles_forward());
  // }
  // _ => (),
  // };
  Ok(())
}

fn predict(symbol: &str, interval: &str, relative: &str) -> Result<()> {
  let mut con = con();
  let step = interval.ms();
  let relative = relative.ms();
  assert!(relative < 0);
  let ms = (now() + relative).round(step);
  // match Frame::new(&mut con, symbol, interval, ms) {
  // Ok(frame) => {
  // frame.predict(CONFIG.predict_candles_forward());
  // }
  // _ => (),
  // }
  let _ = fs::remove_dir_all("builder/csv/predict");
  Ok(())
}
