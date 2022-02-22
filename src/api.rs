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
    let mut tries = 0;
    let mut missing = query.missing_candles()?;

    if missing.is_empty() {
      return query.query_candles();
    }

    while !missing.is_empty() && tries < 3 {
      log!(
        "Fetching {} candles.",
        missing.num_candles(query.interval())
      );

      for range in missing {
        let mut subquery = query.clone();
        // split the range up so we don't get rate-limited
        subquery.set_all(vec![Start(range.start), End(range.end)]);
        log!("missing: {:?}", range);
        let candles = match self {
          Self::Binance(b) => b.fetch_candles(&subquery),
        }?;

        log!("Api returned {} candles.", candles.len());
        let pb_label = "Inserting candles...".to_owned();
        let _ = terminal::PB.0.send((pb_label.clone(), 0.));
        let (mut i, num_candles) = (0f64, candles.len() as f64);

        for candle in candles {
          let _ = terminal::PB.0.send((pb_label.clone(), i / num_candles));
          query.insert_candle(&candle)?;
          i += 1.;
        }
        let _ = terminal::PB.0.send((pb_label.clone(), -1.));
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
