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
  pub fn save_candles(&self, query: &mut Query) -> Result<Vec<Candle>> {
    let step = query.step();
    let range = query.range().expect("Query needs a start and an end.");

    let mut tries = 0;
    let mut missing = query.missing_candles()?;

    while !missing.is_empty() && tries < 3 {
      log!(
        "Fetching {} candles.",
        missing.num_candles(query.interval())
      );

      for range in missing {
        let mut subquery = query.clone();
        // split the range up so we don't get rate-limited
        subquery.set_all(&[Start(range.start), End(range.end)]);
        log!("missing: {:?}", range);
        let candles = match self {
          Self::Binance(b) => b.fetch_candles(&subquery),
        }?;

        log!("Api returned {} candles.", candles.len());
        for candle in candles {
          query.insert_candle(&candle)?;
        }
      }

      tries += 1;
      missing = query.missing_candles()?;
    }

    query.linear_regression()?;
    query.query_candles()
  }
}

// fn linear_regression(q: &Query, candles: &mut [Option<Candle>]) -> Result<()> {
//   let start = q.start().expect("Query needs a start");
//   let step = q.step();
//   if let Some(i) = candles[1..].iter().position(|c| c.is_none()) {
//     let left = &candles[i - 1].expect("Left candle is none");
//     if let Some(right) = candles[i..].iter().position(|c| c.is_some()) {
//       let right = &candles[right].expect("Right candle is none");
//       let c = Candle::linear_regression(start + step * i as i64, left, right)?;
//       candles[i] = Some(c);
//     }
//   }
//   Ok(())
// }
