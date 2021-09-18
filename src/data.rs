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

pub fn build_database() -> Result<()> {
  let mut con = con();
  let mut api = Binance::new();
  let intervals = vec![
    ("15m", 100000i64),
    // ("30m", 10000i64),
    // ("1h", 10000i64),
    // ("4h", 10000i64),
    // ("1d", 1000i64),
  ];
  let symbols = vec!["BTCUSDT"];

  for (interval, count) in intervals {
    for symbol in &symbols {
      let step = interval.to_step()?;

      api.fetch_candles(symbol, interval, now() - count * step, now())?;
    }
  }
  Ok(())
}

pub fn time_defaults(
  start_time: Option<DateTime<Utc>>,
  end_time: Option<DateTime<Utc>>,
  interval: &str,
) -> Result<(i64, i64)> {
  let step = interval.to_step()?;
  let start_ms = match start_time {
    Some(time) => to_ms(&time, interval)?,
    None => match end_time {
      Some(time) => to_ms(&time, interval)? - step * NUM_CANDLES,
      None => Utc::now().timestamp_millis() - step * NUM_CANDLES,
    },
  };
  let end_ms = match end_time {
    Some(time) => to_ms(&time, interval)?,
    None => match start_time {
      Some(time) => to_ms(&time, interval)? + step * NUM_CANDLES,
      None => to_ms(&Utc::now(), interval)? - step,
    },
  };
  Ok((start_ms, end_ms))
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
  use data::time_defaults;

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

    let symbol = "BTCUSDT";
    let interval = "15m";

    let (start_ms, end_ms) = time_defaults(None, None, interval)?;
    println!("start_ms: {}", start_ms);
    println!("end_ms: {}", end_ms);
    let api = Binance::new();
    api.fetch_candles(symbol, interval, start_ms, end_ms)?;

    // database::calculate_domain(&mut con, interval);
    Ok(())
  }

  #[test]
  fn calculates_missing_candles() -> Result<()> {
    test_prep();
    let mut con = con();

    let interval = "15m";
    let step = interval.to_step()?;
    let t1 = Utc.ymd(2017, 12, 12).and_hms(0, 0, 0);
    let t2 = Utc.ymd(2017, 12, 12).and_hms(1, 0, 0);
    let t3 = Utc.ymd(2017, 12, 12).and_hms(2, 0, 0);
    let t4 = Utc.ymd(2017, 12, 12).and_hms(3, 0, 0);

    let t1_ms = data::to_ms(&t1, interval)?;
    let t2_ms = data::to_ms(&t2, interval)?;
    let t3_ms = data::to_ms(&t3, interval)?;
    let t4_ms = data::to_ms(&t4, interval)?;

    let api = Binance::new();

    let result = api.fetch_candles(SYMBOL, interval, t1_ms, t2_ms)?;
    assert_eq!(result.len(), 4);
    assert_eq!(result[0].open_time, t1_ms);
    assert_eq!(
      result.last().unwrap().open_time,
      t2_ms - interval.to_step()?
    );
    let result = api.fetch_candles(SYMBOL, interval, t3_ms, t4_ms)?;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0].open_time, t3_ms);
    assert_eq!(
      result.last().unwrap().open_time,
      t4_ms - interval.to_step()?
    );

    let missing = con.missing_candles(SYMBOL, "15m", t1_ms, t4_ms)?;

    assert_eq!(missing.len(), 1); // one group (groups are split if candles are not adjacent)
    assert_eq!(missing[0].start, t2_ms);
    assert_eq!(missing[0].end, t3_ms - step);
    assert_eq!(missing[0].start, t2_ms); // assert that missing candles are in the window
    assert!(missing[0].end < t3_ms);

    let candles = con.query_candles(
      SYMBOL,
      interval,
      Some(database::QueryOptions {
        start: Some(t1_ms),
        end: Some(t4_ms),
        ..Default::default()
      }),
    )?;

    assert_eq!(candles.len(), 8);

    // fetch the 3 missing candles, and check that they were saved
    let _ = api.fetch_candles(SYMBOL, interval, t1_ms, t4_ms);
    let thirteen_candles = con.query_candles(
      &SYMBOL,
      interval,
      Some(database::QueryOptions {
        start: Some(t1_ms),
        end: Some(t4_ms),
        ..Default::default()
      }),
    )?;

    assert_eq!(thirteen_candles.len(), 12);

    Ok(())
  }
}
