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
        log!("no missing");
        break;
      }

      let missing_count: Vec<usize> =
        missing.iter().map(|m| m.num_candles(step)).collect();
      log!(
        "Fetching {} candles. ({:?})",
        missing_count.iter().sum::<usize>(),
        missing_count
      );

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
    let mut dead_count = 0;
    for range in query.missing_candles()? {
      let mut open_time = range.start;
      while open_time <= range.end {
        // log!("Inserting dead candle at {}..", open_time);
        dead_count += 1;
        let candle = Candle::dead(open_time);
        query
          .insert_candle(&candle)
          .expect("Could not insert candle.");
        open_time += step as i64;
      }
    }
    if dead_count > 0 {
      log!("Inserted {} dead candles.", dead_count);
    }

    Ok(query.query_candles()?)
  }
}
