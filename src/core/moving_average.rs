use postgres::error::SqlState;
use tui::symbols::line::DOUBLE_CROSS;

use crate::prelude::*;

pub static UNIQUE_VIOLATIONS: AtomicUsize = AtomicUsize::new(0);
pub struct MovingAverage {
  symbol: String,
  interval: String,
  ms: i64,
  len: i32, // 240
  val: f32,
  exp: bool,
}

impl MovingAverage {
  pub const DB_COLUMNS: &'static str = "symbol, interval, ms, len, val, exp";

  pub fn calculate_ema(symbol: &str, interval: &str, len: usize) -> Result<()> {
    let q = Query::new(symbol, interval);
    let candles = q.query_candles()?;

    assert!(len < candles.len());

    let step = interval.ms();
    // ensure candles are one continguous chunk
    assert_eq!(
      candles[0].open_time + step * candles.len() as i64 - step,
      candles.last().unwrap().open_time
    );

    let mut ma = candles[..len].iter().fold(0., |acc, c| acc + c.close) as f32
      / len as f32;
    let k = 2. / (len as f32 + 1.);
    let mut result = vec![];

    let len_i32 = len as i32;

    for i in len..candles.len() {
      ma = candles[i].close * k + ma * (1. - k);

      result.push(MovingAverage {
        symbol: symbol.to_owned(),
        interval: interval.to_owned(),
        ms: candles[i].open_time,
        len: len_i32,
        val: ma,
        exp: true,
      });
    }

    log!(
      "Saving MA from {} to {}",
      result[0].ms.to_human(),
      result.last().unwrap().ms.to_human()
    );
    for ma in &result {
      ma.save()?;
    }
    log!("Done");

    Ok(())
  }

  pub fn query(
    symbol: &str,
    interval: &str,
    len: i32,
    exp: bool,
    range: Option<Range<i64>>,
  ) -> Result<Vec<MovingAverage>> {
    let mut query = format!(
      r#"SELECT {} FROM moving_averages WHERE symbol = '{}' AND interval = '{}' AND len = {} AND exp = {}"#,
      Self::DB_COLUMNS,
      symbol,
      interval,
      len,
      exp
    );

    if let Some(range) = range {
      query.push_str(&format!(
        " AND ms >= {} AND ms <= {}",
        range.start, range.end
      ));
    }

    let rows = con().query(query.as_str(), &[])?;
    Ok(rows.iter().map(|r| r.into()).collect())
  }

  fn save(&self) -> Result<()> {
    log!(
      "Saving {} EMA {} for {}, val: {}",
      self.interval,
      self.len,
      self.ms.to_human(),
      self.val
    );

    let result = con().execute(
      "INSERT INTO moving_averages (symbol, interval, ms, len, val, exp) values ($1, $2, $3, $4, $5, $6)",
    &[&self.symbol, &self.interval, &self.ms, &self.len, &self.val, &self.exp]
    );

    if let Err(e) = result {
      match e.code() {
        Some(&SqlState::UNIQUE_VIOLATION) => {
          UNIQUE_VIOLATIONS.fetch_add(1, Relaxed);
        }
        _ => Err(e)?,
      }
    }

    Ok(())
  }
}

impl From<&postgres::Row> for MovingAverage {
  fn from(row: &postgres::Row) -> Self {
    Self {
      symbol: row.get(0),
      interval: row.get(1),
      ms: row.get(2),
      len: row.get(3),
      val: row.get(4),
      exp: row.get(5),
    }
  }
}
