mod binance;
mod ftx;

use crate::prelude::*;
use anyhow::Result;
pub use binance::Binance;

pub trait ApiTrait {
  fn new() -> Api;
  fn fetch_candles(&self, query: &Query) -> Result<Vec<Candle>>;
}

pub enum Api {
  Binance(Binance),
}

impl Api {
  pub fn fetch_candles(&self, query: &mut Query) -> Result<Vec<Candle>> {
    let mut con = con();
    let step = query.step();

    for _ in 0..2 {
      let missing = query.missing_candles()?;

      if missing.is_empty() {
        break;
      }

      for range in missing {
        // split the range up so we don't get rate-limited
        let candles = match self {
          Self::Binance(b) => b.fetch_candles(query),
        }?;
        log!("Api returned {} candles.", candles.len());
        for candle in candles {
          query.insert_candle(&candle)?;
        }
      }
    }

    // Insert remaining missing candles as dead
    for range in query.missing_candles()? {
      let mut open_time = range.start;
      while open_time <= range.end {
        log!("Inserting dead candle at {}..", open_time);
        let candle = Candle::dead(open_time);
        query
          .insert_candle(&candle)
          .expect("Could not insert candle.");
        open_time += step as i64;
      }
    }

    Ok(query.query_candles()?)
  }
}
