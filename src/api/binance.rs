use super::*;
use crate::{prelude::*, Candle};
use serde::{Deserialize, Serialize};

const CANDLE_LIMIT: i64 = 500;

pub struct Binance {}
impl ApiTrait for Binance {
  fn new() -> Api { Api::Binance(Self {}) }
  fn fetch_candles(&self, query: &Query) -> Result<Vec<Candle>> {
    let step = query.step();

    let candle_limit_ms = (CANDLE_LIMIT * step) as usize;
    let expected = query.num_candles();
    let mut candles = Vec::with_capacity(expected as usize);

    let mut fetch = |start: i64, end: i64| -> Result<()> {
      let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&startTime={}&endTime={}",
        query.symbol,
        query.interval,
        start,
        end
      );

      log!("url: {}", url);

      let body = reqwest::blocking::get(&url)?.text()?;
      let raw_candles: Vec<RawCandle> = serde_json::from_str(&body)?;

      candles.extend(
        raw_candles
          .iter()
          .map(Candle::from_raw)
          .collect::<Vec<Candle>>(),
      );

      Ok(())
    };

    let r = query.range().unwrap();
    let r = r.start..(r.end - 1);
    for start in (r.start..r.end).step_by(candle_limit_ms) {
      fetch(start, (start + candle_limit_ms as i64).min(r.end))?;
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
  #[test]
  fn test_api_is_inclusive() -> anyhow::Result<()> {
    // TODO: implement
    // assert!(false);
    // b.fetch_candles();

    Ok(())
  }
}
