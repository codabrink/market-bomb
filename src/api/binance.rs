use super::*;
use crate::{prelude::*, Candle};
use serde::{Deserialize, Serialize};

const CANDLE_LIMIT: i64 = 500;

pub struct Binance {}
impl ApiTrait for Binance {
  fn new() -> Api { Api::Binance(Self {}) }
  fn fetch_candles(
    &self,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Result<Vec<Candle>> {
    if start == end {
      return Ok(vec![]);
    }

    let step = interval.to_step()?;
    let candle_limit_ms = (CANDLE_LIMIT * step) as usize;
    let expected = (start..end).num_candles(step);
    let mut candles = Vec::with_capacity(expected as usize);

    let mut fetch = |start: i64, end: i64| -> Result<()> {
      // if ARGS.is_present("verbose") {
      // println!("Fetching: {} to {}", start, end);
      // }

      let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&startTime={}&endTime={}",
        symbol,
        interval,
        start,
        end
      );

      let body = ureq::get(&url).call()?.into_string()?;
      let raw_candles: Vec<RawCandle> = serde_json::from_str(&body)?;

      // if ARGS.is_present("verbose") {
      // println!("Api responded with {} candles", raw_candles.len());
      // }

      candles.extend(
        raw_candles
          .iter()
          .map(Candle::from_raw)
          .collect::<Vec<Candle>>(),
      );

      Ok(())
    };

    for start in (start..end).step_by(candle_limit_ms) {
      fetch(start, (start + candle_limit_ms as i64).min(end))?;
    }

    Ok(candles)
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RawCandle(
  i64,    // open time
  String, // open price
  String, // high price
  String, // low price
  String, // close price
  String, // volume
  i64,    // close time
  String, // quote asset volume
  i64,    // number of trades
  String,
  String,
  String,
);

trait FromRawCandle<T> {
  fn from_raw(raw: &RawCandle) -> T;
}
impl FromRawCandle<Candle> for Candle {
  fn from_raw(raw: &RawCandle) -> Self {
    Candle {
      open: raw.1.parse().unwrap(),
      high: raw.2.parse().unwrap(),
      low: raw.3.parse().unwrap(),
      close: raw.4.parse().unwrap(),
      volume: raw.5.parse().unwrap(),
      open_time: raw.0,
      close_time: raw.6,
      ..Default::default()
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::api::binance::*;
  #[test]
  fn test_api_is_inclusive() -> anyhow::Result<()> {
    let fifteen_minutes = "15m".to_step()?;
    let b = Binance::new();
    // b.fetch_candles();

    Ok(())
  }
}
