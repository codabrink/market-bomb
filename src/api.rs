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
    let step = query.step();
    let range = query.range().expect("Query needs a start and an end.");

    for _ in 0..2 {
      let missing = query.missing_candles()?;

      if missing.is_empty() {
        log!("no missing");
        break;
      }

      assert!("15m".ms() == step);

      log!(
        "Fetching {} candles. ({:?})",
        missing.num_candles("15m"),
        missing
          .iter()
          .map(|m| m.num_candles("15m"))
          .collect::<Vec<usize>>()
      );

      for range in missing {
        // split the range up so we don't get rate-limited
        query.set_all(&[Start(range.start), End(range.end)]);
        log!("missing: {:?}", range);
        let candles = match self {
          Self::Binance(b) => b.fetch_candles(&query),
        }?;

        log!("Api returned {} candles.", candles.len());
        for candle in candles {
          query.insert_candle(&candle)?;
        }
      }
    }

    query.set_range(range);
    query.linear_regression()?;

    Ok(query.query_candles()?)
  }
}
