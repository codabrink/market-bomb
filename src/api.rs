mod binance;
mod ftx;

use crate::prelude::*;
use anyhow::Result;
pub use binance::Binance;

pub trait ApiTrait {
  fn new() -> Api;
  fn fetch_candles(
    &self,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Result<Vec<Candle>>;
}

pub enum Api {
  Binance(Binance),
}

impl Api {
  pub fn fetch_candles(
    &self,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Result<Vec<Candle>> {
    let mut con = con();
    let step = interval.to_step()?;

    for _ in 0..2 {
      let missing = con.missing_candles(symbol, interval, start, end)?;

      if missing.is_empty() {
        break;
      }

      for range in missing {
        // split the range up so we don't get rate-limited
        let candles = match self {
          Self::Binance(b) => {
            b.fetch_candles(symbol, interval, range.start, range.end)
          }
        }?;
        println!("Api returned {} candles.", candles.len());
        for candle in candles {
          con.insert_candle(symbol, interval, &candle)?;
        }
      }
    }

    // Insert remaining missing candles as dead
    for range in con.missing_candles(symbol, interval, start, end)? {
      let mut open_time = range.start;
      while open_time <= range.end {
        println!("Inserting dead candle at {}..", open_time);
        let candle = Candle::dead(open_time);
        con
          .insert_candle(symbol, interval, &candle)
          .expect("Could not insert candle.");
        open_time += step as i64;
      }
    }

    Ok(con.query_candles(
      symbol,
      interval,
      Some(QueryOptions {
        start: Some(start),
        end: Some(end),
        ..Default::default()
      }),
    )?)
  }
}
