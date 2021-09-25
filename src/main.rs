#[macro_use]
extern crate lazy_static;
extern crate env_logger;
extern crate log;
extern crate r2d2;

mod prelude;

mod api;
mod chart;
mod config;
mod data;
pub mod database;
mod frame;
mod terminal;
mod web_server;

use anyhow::Result;
use chart::*;
use frame::Frame;
use indicatif::ProgressBar;
use prelude::*;
use std::fs;

fn main() { terminal::Terminal::new(); }

fn train(symbol: &str, interval: &str) -> Result<()> {
  // Collect all candles
  data::build_database();
  let mut con = con();

  let folder_path = format!(
    "builder/csv/train/{symbol}/{interval}/{candles_forward}",
    symbol = symbol,
    interval = interval,
    candles_forward = CONFIG.predict_candles_forward()
  );
  let _ = fs::remove_dir_all(&folder_path);
  let _ = fs::remove_file("builder/features.npy");
  let _ = fs::remove_file("builder/labels.npy");

  let step = interval.to_step()?;
  let now = round(now(), step);
  let then = now - CONFIG.history_num_candles * step;

  log!("Writing frames...");
  let pb = ProgressBar::new(((now - then) / step) as u64);
  let mut ms = then;
  while ms < now {
    match Frame::new(&mut con, symbol, interval, ms) {
      Ok(frame) => {
        let _ = frame.write_to_csv(&folder_path);
      }
      Err(e) => {
        log!("{:?}", e);
      }
    }
    pb.inc(1);
    ms += step;
  }
  pb.finish();
  Ok(())
}

fn predict_now(symbol: &str, interval: &str) -> Result<()> {
  data::build_database();

  let mut con = con();
  let _ = fs::remove_dir_all("builder/csv/predict");

  let step = interval.to_step()?;
  let now = round(now(), step);
  match Frame::new(&mut con, symbol, interval, now) {
    Ok(frame) => {
      frame.predict(CONFIG.predict_candles_forward());
    }
    _ => (),
  };
  Ok(())
}

fn predict(symbol: &str, interval: &str, relative: &str) -> Result<()> {
  data::build_database();
  let mut con = con();
  let step = interval.to_step()?;
  let relative = relative.to_step()?;
  assert!(relative < 0);
  let ms = round(now() + relative, step);
  match Frame::new(&mut con, symbol, interval, ms) {
    Ok(frame) => {
      frame.predict(CONFIG.predict_candles_forward());
    }
    _ => (),
  }
  let _ = fs::remove_dir_all("builder/csv/predict");
  Ok(())
}

fn test(symbol: &str, interval: &str) {
  let candles = con().query_candles(symbol, interval, None).unwrap();
  let eighty_percent = (candles.len() as f32 * 0.8) as usize;
}
