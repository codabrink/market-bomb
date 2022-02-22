use crate::prelude::*;

// Things to track...
// 1. Distance from MA/EMA
//   - On a percent of the price multiplied by a constant
// 2. Tweak these values as needed
//   - 3y of monthly candles
//   - 6m of weekly candles
//   - 1m of daily candles
//   - 1w of hourly
//   - 2d of 15m

pub struct MA {
  len: i32,
  interval: String,
  exp: bool,
}
pub struct CandleSegment {
  len: String,
  interval: String,
}
pub struct NormalizedData {
  open: f32,
  close: f32,
  high: f32,
  low: f32,
}

pub fn normalize(
  symbol: &str,
  mut cursor: i64,
  segments: Vec<CandleSegment>,
  moving_averages: Vec<MA>,
) -> Result<Vec<NormalizedData>> {
  let mut candles = vec![];

  // =====================
  // TODO: Gather MA
  // =====================

  // Gather the candle data
  for segment in segments {
    let mut query = Query::new(symbol, &segment.interval);
    let len = segment.len.ms();
    query.set_range((cursor - len)..cursor);

    candles.append(&mut API.save_candles(&mut query)?);
    cursor -= len;
  }

  let c = &candles[0];
  let (max, min) = candles.iter().fold((c.high, c.low), |(max, min), c| {
    (max.max(c.high), min.min(c.low))
  });

  // get the range of values
  let r = max - min;
  // normalize the candles
  let ncd = candles
    .iter()
    .map(|c| NormalizedData {
      open: r / (c.open - min),
      close: r / (c.close - min),
      high: r / (c.high - min),
      low: r / (c.low - min),
    })
    .collect();

  Ok(ncd)
}
