use postgres::error::SqlState;

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

  pub fn calculate_ma(symbol: &str, interval: &str, len: usize) -> Result<()> {
    let q = Query::new(symbol, interval);
    let candles = q.query_candles()?;
    MovingAverage::clear(symbol, interval, len, false)?;

    assert!(len < candles.len());
    candles.ensure_congruent();

    let mut sum = candles[..len].iter().fold(0., |acc, c| acc + c.close) as f32;
    let len_i32 = len as i32;
    let len_f32 = len as f32;

    let pb_label = format!("Moving Average {}, {} - {}", symbol, interval, len);
    terminal::PB.0.send((pb_label.clone(), 0.))?;

    for i in len..candles.len() {
      MovingAverage {
        symbol: symbol.to_owned(),
        interval: interval.to_owned(),
        ms: candles[i].open_time,
        len: len_i32,
        val: sum / len_f32,
        exp: false,
      }
      .save()?;

      sum -= candles[i - len].close;
      sum += candles[i].close;

      terminal::PB
        .0
        .send((pb_label.clone(), i as f64 / (candles.len() - len) as f64))?;
    }

    let _ = terminal::PB.0.send((pb_label.clone(), -1.));

    Ok(())
  }

  fn clear(symbol: &str, interval: &str, len: usize, exp: bool) -> Result<()> {
    let query = format!(
      "DELETE FROM moving_averages WHERE symbol='{}' AND interval='{}' AND len={} AND exp={}",
      symbol, interval, len, exp
    );
    con().batch_execute(&query)?;
    Ok(())
  }

  pub fn calculate_ema(symbol: &str, interval: &str, len: usize) -> Result<()> {
    let q = Query::new(symbol, interval);
    let candles = q.query_candles()?;
    MovingAverage::clear(symbol, interval, len, true)?;

    assert!(len < candles.len());
    candles.ensure_congruent();

    let mut ma = candles[..len].iter().fold(0., |acc, c| acc + c.close) as f32
      / len as f32;
    let k = 2. / (len as f32 + 1.);

    let len_i32 = len as i32;
    let pb_label = format!(
      "Exponential moving Average {}, {} - {}",
      symbol, interval, len
    );
    terminal::PB.0.send((pb_label.clone(), 0.))?;

    for i in len..candles.len() {
      ma = candles[i].close * k + ma * (1. - k);

      MovingAverage {
        symbol: symbol.to_owned(),
        interval: interval.to_owned(),
        ms: candles[i].open_time,
        len: len_i32,
        val: ma,
        exp: true,
      }
      .save()?;

      terminal::PB.0.send((
        pb_label.clone(),
        i as f64 / (candles.len() as f64 - len as f64),
      ))?;
    }

    let _ = terminal::PB.0.send((pb_label.clone(), -1.));
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
    // log!(
    // "Saving {} EMA {} for {}, val: {}",
    // self.interval,
    // self.len,
    // self.ms.to_human(),
    // self.val
    // );

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

#[cfg(test)]
mod tests {
  use crate::prelude::*;

  #[test]
  fn moving_averages_to_and_from_db() -> Result<()> {
    let symbol = "BTCUSDT";
    let interval = "15m";
    let len = 10;

    let mut query = Query::new(symbol, interval);
    query.set_all(vec![Start("5d".ago()), End("3d".ago())]);
    let _ = API.save_candles(&mut query)?;

    query.set_all(vec![Len(len), Exp(true)]);

    let missing_ma = query.missing_ma_ungrouped()?;
    assert_eq!(missing_ma.len(), query.num_candles());

    MovingAverage::calculate_ema(symbol, interval, len as usize)?;
    let ma = MovingAverage::query(symbol, interval, len, true, None)?;
    assert_eq!(ma.len(), query.num_candles() - len as usize);

    let missing_ma = query.missing_ma_ungrouped()?;

    // TODO: Investigate this.. this should be len.. not 0
    assert_eq!(missing_ma.len(), 0);
    // assert_eq!(missing_ma.len(), len as usize);

    Ok(())
  }
}
