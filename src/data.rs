pub use crate::{api::Binance, prelude::*};
pub use anyhow::Result;
pub use chrono::{prelude::*, Utc};
pub use indicatif::{ProgressBar, ProgressStyle};
pub use std::{fs, io::Write, path::Path};

const NUM_CANDLES: i64 = 4000;

pub fn progress_bar(size: i64) -> ProgressBar {
  let pb = ProgressBar::new(size as u64);
  pb.set_style(ProgressStyle::default_bar().progress_chars("#>-"));
  pb
}

pub fn _get_symbols() -> Result<()> {
  use serde::Deserialize;
  if Path::new("data/symbols.json").exists() {
    return Ok(());
  }

  #[derive(Deserialize)]
  struct ExchangeInfo {
    symbols: Vec<String>,
  }

  let json: ExchangeInfo =
    ureq::get("https://api.binance.com/api/v1/exchangeInfo")
      .call()?
      .into_json()?;
  let pretty_json = serde_json::to_string_pretty(&json.symbols)?;

  let mut output = fs::File::create("data/symbols.json")?;
  let _ = write!(output, "{:}", pretty_json);

  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::{data, database, prelude::*};
  use anyhow::Result;
  use chrono::prelude::*;
  use chrono::Utc;

  const SYMBOL: &str = "BTCUSDT";

  #[test]
  fn to_ms_cleans_datetime() -> Result<()> {
    let date = Utc.ymd(2017, 12, 12).and_hms(12, 11, 12);
    assert_eq!(data::to_ms(&date, &"15m")?, 1513080000000);
    let date = Utc.ymd(2017, 12, 12).and_hms(12, 14, 15);
    assert_eq!(data::to_ms(&date, &"15m")?, 1513080000000);
    let date = Utc.ymd(2017, 12, 12).and_hms(12, 16, 15);
    assert_eq!(data::to_ms(&date, &"15m")?, 1513080900000);
    Ok(())
  }

  #[test]
  fn calculates_step() -> Result<()> {
    assert_eq!("15m".to_step()?, 900000);
    assert_eq!("1h".to_step()?, 3600000);
    assert_eq!("1d".to_step()?, 86400000);
    Ok(())
  }

  #[test]
  fn calculates_domain() -> Result<()> {
    test_prep();
    // TODO: rewrite the test
    Ok(())
  }

  #[test]
  fn calculates_missing_candles() -> Result<()> {
    test_prep();
    let mut query = Query::new(SYMBOL, "15m");
    let step = query.step();
    let t1 = Utc.ymd(2017, 12, 12).and_hms(0, 0, 0);
    let t2 = Utc.ymd(2017, 12, 12).and_hms(1, 0, 0);
    let t3 = Utc.ymd(2017, 12, 12).and_hms(2, 0, 0);
    let t4 = Utc.ymd(2017, 12, 12).and_hms(3, 0, 0);

    let t1_ms = data::to_ms(&t1, query.interval)?;
    let t2_ms = data::to_ms(&t2, query.interval)?;
    let t3_ms = data::to_ms(&t3, query.interval)?;
    let t4_ms = data::to_ms(&t4, query.interval)?;

    let api = Binance::new();

    query.set_all(&[Start(t1_ms), End(t2_ms)]);
    let result = api.fetch_candles(&mut query)?;
    assert_eq!(result.len(), 4);
    assert_eq!(result[0].open_time, t1_ms);
    assert_eq!(result.last().unwrap().open_time, t2_ms - query.step());
    query.set_all(&[Start(t3_ms), End(t4_ms)]);
    let result = api.fetch_candles(&mut query)?;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0].open_time, t3_ms);
    assert_eq!(result.last().unwrap().open_time, t4_ms - query.step());

    query.set_all(&[Start(t1_ms), End(t4_ms)]);
    let missing = query.missing_candles()?;

    assert_eq!(missing.len(), 1); // one group (groups are split if candles are not adjacent)
    assert_eq!(missing[0].start, t2_ms);
    assert_eq!(missing[0].end, t3_ms - step);
    assert_eq!(missing[0].start, t2_ms); // assert that missing candles are in the window
    assert!(missing[0].end < t3_ms);

    let candles = query.query_candles()?;

    assert_eq!(candles.len(), 8);

    // fetch the 3 missing candles, and check that they were saved
    let _ = api.fetch_candles(&mut query);
    let thirteen_candles = query.query_candles()?;

    assert_eq!(thirteen_candles.len(), 12);

    Ok(())
  }
}
