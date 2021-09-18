use crate::chart::{Candles, Line, TrendLine};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub enum CrossType {
  REJECT,
  UP,
  DOWN,
  BOUNCE,
  VOID,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct LineCross {
  pub width: i32,
  pub height: f32,
  pub open_index: usize,
  pub close_index: usize,
  pub t: CrossType,
  pub p1: (f32, f32),
  pub p2: (f32, f32),
}

pub fn generate_crosses(
  candles: &Arc<Candles>,
  trend_line: &TrendLine,
  limit: Option<usize>,
) -> Vec<LineCross> {
  let limit = match limit {
    Some(limit) => limit,
    None => candles.len() - 1,
  };
  crawl(
    candles,
    trend_line,
    trend_line.line.roots[1].candle_index + 1,
    vec![],
    limit,
  )
}

fn crawl(
  candles: &Arc<Candles>,
  trend_line: &TrendLine,
  index: usize,
  mut crosses: Vec<LineCross>,
  limit: usize,
) -> Vec<LineCross> {
  if index > limit {
    return crosses;
  }
  match candles.get(index) {
    Some(candle) => {
      match trend_line.line.intersects(candle) {
        true => {
          // cross found, collect the cross
          let cross = collect(candles, &trend_line.line, index, index + 1, 0);
          let close_index = cross.close_index;
          crosses.push(cross);
          crawl(candles, trend_line, close_index + 1, crosses, limit)
        }
        false => crawl(candles, trend_line, index + 1, crosses, limit), // keep crawling
      }
    }
    None => crosses, // done
  }
}

const GAP: usize = 1;
pub fn collect(
  candles: &Arc<Candles>,
  line: &Line,
  open_index: usize,
  close_index: usize,
  gap: usize,
) -> LineCross {
  let create = || -> LineCross {
    let start_candle = candles[open_index];
    let end_candle = candles[close_index - 1];
    let open = start_candle.open - line.y_at_x(start_candle.open_time as f32);
    let close = end_candle.close - line.y_at_x(end_candle.close_time as f32);
    LineCross {
      width: (close_index - open_index) as i32,
      height: 0f32,
      open_index,
      close_index,
      p1: (
        start_candle.open_time as f32,
        line.y_at_x(start_candle.open_time as f32),
      ),
      p2: (
        end_candle.open_time as f32,
        line.y_at_x(end_candle.open_time as f32),
      ),
      t: match (open, close) {
        (o, c) if o <= 0f32 && c <= 0f32 => CrossType::REJECT,
        (o, c) if o <= 0f32 && c > 0f32 => CrossType::UP,
        (o, c) if o > 0f32 && c <= 0f32 => CrossType::DOWN,
        (o, c) if o > 0f32 && c > 0f32 => CrossType::BOUNCE,
        (_, _) => CrossType::VOID,
      },
    }
  };

  match candles.get(close_index + gap) {
    Some(candle) => {
      let intersects = line.intersects(candle);
      match intersects {
        true => collect(candles, line, open_index, close_index + 1, 0),
        false if gap < GAP => {
          collect(candles, line, open_index, close_index, gap + 1)
        }
        false => create(),
      }
    }
    None => create(),
  }
}
