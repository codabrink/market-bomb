use crate::prelude::*;
use anyhow::Result;
use postgres::Row;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct Candle {
  pub id: i32,
  pub open: f32,
  pub high: f32,
  pub low: f32,
  pub close: f32,
  pub volume: f32,
  pub open_time: i64,
  pub close_time: i64,
  pub top_domain: i32,
  pub bottom_domain: i32,
  pub derived: bool,
  pub fuzzy_domain: bool,
  pub index: usize,
}

impl Candle {
  pub const DB_COLUMNS: &'static str = "id, open_time, open, high, low, close, volume, close_time, bottom_domain, top_domain, fuzzy_domain, derived";

  pub fn contains_ms(&self, ms: i64) -> bool {
    self.open_time <= ms && self.close_time >= ms
  }

  pub fn linear_regression(
    open_time: i64,
    left: &Candle,
    right: &Candle,
  ) -> Result<Self> {
    let dl = (open_time - left.open_time) as f32;
    let dr = (right.open_time - open_time) as f32;
    let dt = dl + dr;

    // get the fractions
    let dl = 1. - (dl / dt);
    let dr = 1. - (dr / dt);

    let candle = Candle {
      open: left.open * dl + right.open * dr,
      high: left.high * dl + right.high * dr,
      low: left.low * dl + right.low * dr,
      close: left.close * dl + right.close * dr,
      volume: left.volume * dl + right.volume * dr,
      open_time,
      close_time: open_time + (left.close_time - left.open_time),
      ..Default::default()
    };

    Ok(candle)
  }

  pub fn wick_ratio(&self) -> f32 {
    let bhigh = self.open.max(self.close);
    let blow = self.open.min(self.close);
    let top_wick = self.high - bhigh;
    let bottom_wick = blow - self.low;
    let wick = top_wick + bottom_wick;

    if wick == 0.0 {
      return 0.0;
    }

    top_wick / wick - bottom_wick / wick
  }
  pub fn open_y(&self) -> f32 {
    self.open
  }
  pub fn open_x(&self) -> i64 {
    self.open_time
  }
  pub fn to_string(&self, symbol: &str, interval: &str) -> String {
    format!(
      "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
      self.id,
      symbol,
      interval,
      self.open_time,
      self.open,
      self.high,
      self.low,
      self.close,
      self.volume,
      self.close_time,
      self.bottom_domain,
      self.top_domain,
      self.fuzzy_domain,
      self.derived,
    )
  }
}

impl From<&Row> for Candle {
  fn from(row: &Row) -> Self {
    Self {
      id: row.get(0),
      open_time: row.get(1),
      open: row.get(2),
      high: row.get(3),
      low: row.get(4),
      close: row.get(5),
      volume: row.get(6),
      close_time: row.get(7),
      bottom_domain: row.get(8),
      top_domain: row.get(9),
      fuzzy_domain: row.get(10),
      derived: row.get(11),
      ..Default::default()
    }
  }
}
impl From<(usize, &Row)> for Candle {
  fn from((i, r): (usize, &Row)) -> Self {
    let mut c = Candle::from(r);
    c.index = i;
    c
  }
}

pub fn build_domain(query: &mut Query) -> Result<()> {
  log!(
    "Calculating domain for {}, {}...",
    query.symbol(),
    query.interval()
  );
  let mut candles = query.query_candles()?;
  // let pb = data::progress_bar(candles.len() as i64);

  // pb.finish_and_clear();
  log!("Saving..");

  for i in 0..candles.len() {
    set_domain(&mut candles, i, query.symbol(), query.interval());
  }
  let candle_rows: Vec<String> = candles
    .iter()
    .map(|c| c.to_string(query.symbol(), query.interval()))
    .collect();
  query.copy_in_candles(candle_rows.join("")).unwrap();

  Ok(())
}

fn set_domain(
  candles: &mut [Candle],
  i: usize,
  symbol: &str,
  interval: &str,
) -> String {
  use std::cmp::{max, min};

  let candle = &candles[i];
  // 0: top-left, 1: top-right, 2: bottom-right, 3: bottom-left
  let mut domains = vec![0usize; 4];
  let mut fuzzy_domain = false;

  let mut index = i;

  // look left
  while index > 0 && (domains[0] == 0 || domains[3] == 0) {
    index -= 1;
    let c = &candles[index];

    if domains[0] == 0 && c.high > candle.high {
      domains[0] = i - index;
    }
    if domains[3] == 0 && c.low < candle.low {
      domains[3] = i - index;
    }
  }

  index = i;
  let candles_len = candles.len() - 1;

  // look right
  while index < candles_len && (domains[1] == 0 || domains[2] == 0) {
    index += 1;
    let c = &candles[index];

    if domains[1] == 0 && c.high >= candle.high {
      domains[1] = index - i;
    }
    if domains[2] == 0 && c.low <= candle.low {
      domains[2] = index - i;
    }
  }

  let mut maybe_fuzzy = |a, b| {
    let mut domain = max(a, b);
    if domain == 0 {
      // all time high / low
      domain = min(max(i, 2) - 1, candles_len - i);
    }
    let distance_from_edge = min(max(i, 2) - 1, candles_len - i);
    if domain >= distance_from_edge {
      fuzzy_domain = true;
    }
    domain
  };

  candles[i].top_domain = match (domains[2], domains[3]) {
    (0, _) | (_, 0) => maybe_fuzzy(domains[2], domains[3]),
    (a, b) => min(a, b),
  } as i32;
  candles[i].bottom_domain = match (domains[0], domains[1]) {
    (0, _) | (_, 0) => maybe_fuzzy(domains[0], domains[1]),
    (a, b) => min(a, b),
  } as i32;
  candles[i].fuzzy_domain = fuzzy_domain;
  candles[i].to_string(symbol, interval)
}
