use crate::prelude::*;

pub struct MovingAverage {
  symbol: String,
  interval: String,
  ms: i64,
  len: i32, // 240
  val: f32,
  exponential: bool,
}

impl MovingAverage {
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
        exponential: true,
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
  fn save(&self) -> Result<()> {
    log!(
      "Saving {} EMA {} for {}, val: {}",
      self.interval,
      self.len,
      self.ms.to_human(),
      self.val
    );
    con().execute(
      "INSERT INTO moving_averages (symbol, interval, ms, len, val, exponential) values ($1, $2, $3, $4, $5, $6)",
    &[&self.symbol, &self.interval, &self.ms, &self.len, &self.val, &self.exponential]
    );

    Ok(())
  }
}
