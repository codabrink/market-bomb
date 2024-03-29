use super::*;
use crate::prelude::*;
use serde::{Deserialize, Serialize};

const CANDLE_LIMIT: i64 = 500;

pub struct Binance {}
impl ApiTrait for Binance {
  fn new() -> Api {
    Api::Binance(Self {})
  }
  fn fetch_candles(&self, query: &Query) -> Result<Vec<Candle>> {
    let step = query.step();
    let fetch_step = (CANDLE_LIMIT * step) as usize;
    let mut result = Vec::with_capacity(query.num_candles());

    let mut fetch = |start: i64, end: i64| -> Result<()> {
      let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&startTime={}&endTime={}",
        query.symbol(),
        query.interval(),
        start,
        end
      );

      log!("url: {}", url);

      let body = reqwest::blocking::get(&url)?.text()?;
      let raw_candles: Vec<RawCandle> = serde_json::from_str(&body)?;

      for rc in raw_candles {
        result.push(rc.into());
      }

      Ok(())
    };

    let pb_label = "Downloading candles...".to_owned();
    let _ = terminal::PB.0.send((pb_label.clone(), 0.));

    let r = query.range().unwrap();
    let r = r.start..(r.end - 1);
    for start in (r.start..r.end).step_by(fetch_step) {
      let _ = terminal::PB.0.send((
        pb_label.clone(),
        (start - r.start) as f64 / (r.end - r.start) as f64,
      ));
      fetch(start, (start + fetch_step as i64).min(r.end))?;
    }

    let _ = terminal::PB.0.send((pb_label.clone(), -1.));

    Ok(result)
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

impl From<RawCandle> for Candle {
  fn from(rc: RawCandle) -> Candle {
    Candle {
      open: rc.1.parse().unwrap(),
      high: rc.2.parse().unwrap(),
      low: rc.3.parse().unwrap(),
      close: rc.4.parse().unwrap(),
      volume: rc.5.parse().unwrap(),
      open_time: rc.0,
      close_time: rc.6,
      ..Default::default()
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::prelude::*;

  #[test]
  fn test_api_is_inclusive() -> Result<()> {
    let api = Binance::new();
    let mut query = Query::default();

    query.set_all(vec![Start("4h".ago()), End("3h".ago())]);
    assert_eq!(query.end().unwrap() - query.start().unwrap(), "1h".ms());

    assert_eq!(query.missing_candles_ungrouped()?.len(), 4);
    assert_eq!(query.missing_candles()?[0].num_candles("15m"), 4);

    let candles = api.save_candles(&mut query)?;
    for c in &candles {
      log!("time: {}", c.open_time.to_human());
    }
    assert_eq!(candles.len(), 4);

    Ok(())
  }

  #[test]
  fn week_interval_works() -> Result<()> {
    let api = Binance::new();
    let mut query = Query::new("BTCUSDT", "1w");
    query.set_all(vec![Start("20w".ago()), End(now())]);

    let candles = api.save_candles(&mut query)?;
    assert_eq!(candles.len(), 20);

    Ok(())
  }
}
