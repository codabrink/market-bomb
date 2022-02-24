use crate::prelude::*;

// Things to track...
// 1. Distance from MA/EMA
//   - On a percent of the price multiplied by a constant
// 2. Tweak these values as needed
//   - 2y of weekly candles
//   - 4w of daily candles
//   - 1w of 4h candles
//   - 4d of hourly
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
  ma: Vec<f32>,
}

const MA_MULT: f32 = 8.;

pub fn normalize(
  symbol: &str,
  mut cursor: i64,
  segments: Vec<CandleSegment>,
  moving_averages: Vec<MA>,
) -> Result<Vec<NormalizedData>> {
  let mut data = vec![];

  // Gather the candle data
  for segment in segments {
    let mut query = Query::new(symbol, &segment.interval);
    let len = segment.len.ms();
    query.set_range((cursor - len)..cursor);

    let candles = API.save_candles(&mut query)?;
    for candle in candles {
      let ma_prices: Vec<f32> = moving_averages
        .iter()
        .map(|ma| {
          let val = query
            .ma_price(symbol, &ma.interval, candle.open_time, ma.len, ma.exp)
            .expect("Do not have MA price.");
          (val - candle.close) / candle.close * MA_MULT
        })
        .collect();

      data.push((candle, ma_prices))
    }

    cursor -= len;
  }

  let (c, _) = &data[0];
  let (max, min) = data.iter().fold((c.high, c.low), |(max, min), (c, _)| {
    (max.max(c.high), min.min(c.low))
  });

  // get the range of values
  let r = max - min;
  // normalize the data
  let ncd = data
    .into_iter()
    .map(|(c, ma)| NormalizedData {
      open: r / (c.open - min),
      close: r / (c.close - min),
      high: r / (c.high - min),
      low: r / (c.low - min),
      ma,
    })
    .collect();

  Ok(ncd)
}
