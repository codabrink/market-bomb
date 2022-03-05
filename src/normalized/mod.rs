use crate::prelude::*;

mod strat1;

// Things to track...
// 1. Distance from MA/EMA
//   - On a percent of the price multiplied by a constant
// 2. Tweak these values as needed
//   - 2y of weekly candles
//   - 4w of daily candles
//   - 1w of 4h candles
//   - 4d of hourly
//   - 2d of 15m

// MAD - Moving Average Description
#[derive(Clone)]
pub struct MAD {
  pub len: i32,
  pub interval: String,
  pub exp: bool,
}
impl From<&str> for MAD {
  fn from(input: &str) -> Self {
    let pieces: Vec<&str> = input.split(":").collect();
    Self {
      interval: pieces[0].parse().unwrap(),
      len: pieces[1].parse().unwrap(),
      exp: pieces[2].parse().unwrap(),
    }
  }
}
#[derive(Clone)]
pub struct CandlesChunkDesc {
  pub len: String,
  pub interval: String,
}

impl CandlesChunkDesc {
  pub fn new(len: &str, interval: &str) -> Self {
    Self {
      len: len.to_string(),
      interval: interval.to_string(),
    }
  }
}
#[derive(Clone)]
pub struct Frame {
  ms: i64,
  open: f32,
  close: f32,
  high: f32,
  low: f32,
  ma: Vec<f32>,
}

pub trait StratStr<'a> {
  fn to_components(&'a self) -> (Vec<&'a str>, Vec<MAD>);
  fn load(&self, symbol: &str, cursor: i64) -> Result<Vec<Frame>>;
  fn strat_len(&self) -> i64;
}
impl<'a> StratStr<'a> for &'a str {
  fn to_components(&self) -> (Vec<&str>, Vec<MAD>) {
    let [candle_strat, ma_strat] = (self.split(";").collect::<Vec<&str>>())[..];
    let ma_strats: Vec<&str> = ma_strat.split(",").collect();
    let moving_averages: Vec<MAD> =
      ma_strats.into_iter().map(|ma| ma.into()).collect();
    let candle_strats: Vec<&str> = candle_strat.split(",").collect();

    (candle_strats, moving_averages)
  }
  fn strat_len(&self) -> i64 {
    let (candle_strats, _) = self.to_components();
    candle_strats.into_iter().fold(0, |acc, s| {
      let [len, interval] = (s.split(":").collect::<Vec<&str>>())[..];
      acc + len.ms() * interval.ms()
    })
  }
  fn load(&self, symbol: &str, mut cursor: i64) -> Result<Vec<Frame>> {
    let mut frames = vec![];
    let (candle_strats, moving_averages) = self.to_components();

    for strat in candle_strats {
      let [len, interval] = (strat.split(":").collect::<Vec<&str>>())[..];
      let mut query = Query::new(symbol, interval);
      let len = len.ms();

      query.set_range((cursor - len)..cursor);
      let candles = API.save_candles(&mut query)?;

      // TODO: Figure out why this is so
      assert_eq!(candles.len() - 1, query.num_candles());

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

        frames.push(Frame {
          ms: candle.open_time,
          open: candle.open,
          close: candle.close,
          high: candle.high,
          low: candle.low,
          ma: ma_prices,
        });
      }
    }

    Ok(frames)
  }
}

pub trait ExportData {
  fn export(&self, file: &mut File, label: f32) -> Result<()>;
}
impl ExportData for Vec<Frame> {
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

fn collect(
  symbol: &str,
  mut cursor: i64,
  chunks: Vec<CandlesChunkDesc>,
  moving_averages: Vec<MAD>,
) -> Result<Vec<Frame>> {
  let mut frames = vec![];

  // Gather the candle data
  for chunk in chunks {
    let mut query = Query::new(symbol, &chunk.interval);
    let len = chunk.len.ms();
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

      frames.push(Frame {
        ms: candle.open_time,
        open: candle.open,
        close: candle.close,
        high: candle.high,
        low: candle.low,
        ma: ma_prices,
      });
    }

    cursor -= len;
  }

  Ok(frames)
}

pub fn normalize(
  symbol: &str,
  mut cursor: i64,
  segments: Vec<CandlesChunkDesc>,
  moving_averages: Vec<MAD>,
) -> Result<Vec<Frame>> {
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
    .map(|(c, ma)| Frame {
      ms: c.open_time,
      open: (c.open - min) / r,
      close: (c.close - min) / r,
      high: (c.high - min) / r,
      low: (c.low - min) / r,
      ma,
    })
    .collect();

  Ok(ncd)
}
