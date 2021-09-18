pub mod candle;
pub mod candles;
pub mod indicators;
// pub mod line_cross;
pub mod strong_point;
// pub mod trend_line;

pub(crate) use self::candle::Candle;
pub(crate) use self::candles::Candles;
pub(crate) use self::strong_point::StrongPoint;

use crate::prelude::*;
use serde::Serialize;
use std::sync::Arc;

impl Chart {
  pub fn new(
    api: &mut Api,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Chart {
    let candles = api.fetch_candles(symbol, interval, start, end).unwrap();

    let candles = Arc::new(Candles::new(candles));

    let strong_points = strong_point::generate_points(&candles.candles);

    // let trend_lines = trend_line::generate_bounding_lines(&candles);

    let candles = Arc::try_unwrap(candles).unwrap();

    Chart {
      meta: ChartMeta {
        symbol: symbol.to_string(),
        interval: interval.to_string(),
        step: interval.to_step().unwrap(),
        start,
        end,
      },
      candles,
      strong_points,
    }
  }
}

#[derive(Default, Debug, Serialize)]
pub struct Chart {
  meta: ChartMeta,
  candles: Candles,
  strong_points: Vec<StrongPoint>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct ChartMeta {
  interval: String,
  symbol: String,
  step: i64,
  start: i64,
  end: i64,
}
