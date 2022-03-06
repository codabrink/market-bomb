use crate::prelude::*;
use serde::Serialize;

use CandlePos::*;

#[derive(Serialize, Clone, Debug, PartialEq, Copy)]
pub enum CandlePos {
  HIGH,
  LOW,
  // OPEN,
  // CLOSE,
}
impl From<CandlePos> for u8 {
  fn from(cp: CandlePos) -> Self {
    match cp {
      HIGH => 1,
      LOW => 0,
    }
  }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct StrongPoint {
  pub position: CandlePos,
  pub x: i64,
  pub y: f32,
  pub index: usize,
  pub candle_index: usize,
  pub domain: i32,
}

#[derive(Debug, Serialize, Clone)]
pub struct StrongPointConfig {
  pub min_domain: u16,
  pub count: u16,
}

impl StrongPoint {
  pub fn new(
    candles: &[Candle],
    candle_index: usize,
    position: CandlePos,
    index: usize,
  ) -> StrongPoint {
    let candle = &candles[candle_index];

    StrongPoint {
      x: candle.open_time as i64,
      y: match position {
        CandlePos::HIGH => candle.high,
        CandlePos::LOW => candle.low,
      } as f32,
      position,
      domain: match position {
        CandlePos::HIGH => candle.top_domain,
        CandlePos::LOW => candle.bottom_domain,
      },
      candle_index,
      index,
    }
  }
}

pub fn generate_points(candles: &[Candle]) -> Vec<StrongPoint> {
  let min_domain = CONFIG.strong_points.min_domain;
  let mut strong_points = vec![];
  let mut index = 0;

  for i in 0..candles.len() {
    let candle = &candles[i];
    if candle.top_domain >= min_domain {
      strong_points.push(StrongPoint::new(candles, i, HIGH, index));
      index += 1;
    }
    if candle.bottom_domain >= min_domain {
      strong_points.push(StrongPoint::new(candles, i, LOW, index));
      index += 1;
    }
  }

  strong_points
}
