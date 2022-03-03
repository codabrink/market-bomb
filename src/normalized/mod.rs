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

#[derive(Clone)]
pub struct MA {
  pub len: i32,
  pub interval: String,
  pub exp: bool,
}
#[derive(Clone)]
pub struct CandleSegment {
  pub len: String,
  pub interval: String,
}

impl CandleSegment {
  pub fn new(len: &str, interval: &str) -> Self {
    Self {
      len: len.to_string(),
      interval: interval.to_string(),
    }
  }
}
#[derive(Clone)]
pub struct NormalizedData {
  open: f32,
  close: f32,
  high: f32,
  low: f32,
  ma: Vec<f32>,
}

pub trait ExportData {
  fn export(&self, file: &mut File, label: f32) -> Result<()>;
}
impl ExportData for Vec<NormalizedData> {
  fn export(&self, file: &mut File, label: f32) -> Result<()> {
    let mut result = vec![];
    for d in self {
      let ma: Vec<String> = d.ma.iter().map(|ma| ma.to_string()).collect();

      result.push(format!(
        "{},{},{},{},{}",
        d.open,
        d.close,
        d.high,
        d.low,
        ma.join(",")
      ));
    }

    write!(file, "{}", result.join(","))?;
    let label = match label {
      l if l > 0.02 => "pos",
      l if l < -0.02 => "neg",
      _ => "flt",
    };
    writeln!(file, ",{}", label)?;

    Ok(())
  }
}

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
    // todo: bad. Fix.
    if candles.len() - 1 != query.num_candles() {
      bail!("Mismatched candle num.");
    }

    for candle in candles {
      let ma_prices: Vec<f32> = moving_averages
        .iter()
        .map(|ma| {
          let val = query
            .ma_price(symbol, &ma.interval, candle.open_time, ma.len, ma.exp)
            .expect("Do not have MA price.");
          (val - candle.close) / candle.close
        })
        .collect();

      assert_eq!(ma_prices.len(), moving_averages.len());

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
      open: (c.open - min) / r,
      close: (c.close - min) / r,
      high: (c.high - min) / r,
      low: (c.low - min) / r,
      ma,
    })
    .collect();

  Ok(ncd)
}
