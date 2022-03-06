use crate::prelude::*;

pub mod strat1;

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
    if let [candle_strat, ma_strat] =
      (self.split(";").collect::<Vec<&str>>())[..]
    {
      let ma_strats: Vec<&str> = ma_strat.split(",").collect();
      let moving_averages: Vec<MAD> =
        ma_strats.into_iter().map(|ma| ma.into()).collect();
      let candle_strats: Vec<&str> = candle_strat.split(",").collect();

      (candle_strats, moving_averages)
    } else {
      panic!("Strat str is malformed");
    }
  }
  fn strat_len(&self) -> i64 {
    let (candle_strats, _) = self.to_components();
    candle_strats.into_iter().fold(0, |acc, s| {
      match (s.split(":").collect::<Vec<&str>>())[..] {
        [len, _] => acc + len.ms(),
        _ => unreachable!(),
      }
    })
  }
  fn load(&self, symbol: &str, cursor: i64) -> Result<Vec<Frame>> {
    let mut frames = vec![];
    let (candle_strats, moving_averages) = self.to_components();

    for strat in candle_strats {
      let [len, interval] = match (strat.split(":").collect::<Vec<&str>>())[..]
      {
        [a, b] => [a, b],
        _ => bail!("Strat str is malformed."),
      };
      let mut query = Query::new(symbol, interval);
      let len = len.ms();

      query.set_range((cursor - len)..cursor);
      let candles = API.save_candles(&mut query)?;

      // TODO: Figure out why this is so
      if candles.len() - 1 != query.num_candles() {
        bail!("Data is no good.");
      }

      for candle in candles {
        let ma_prices: Vec<f32> = moving_averages
          .iter()
          .map(|ma| {
            query
              .ma_price(symbol, &ma.interval, candle.open_time, ma.len, ma.exp)
              .expect("Do not have MA price.")
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
