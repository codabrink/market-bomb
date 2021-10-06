use crate::prelude::*;

pub struct MovingAverage {
  symbol: String,
  interval: String,
  ms: i64,
  len: i32,
  val: f32,
  exponential: bool,
}

impl MovingAverage {
  fn save(&mut self) -> Result<()> {
    let step = self.interval.as_str().to_step()?;
    let ms = self.ms.round(step);

    let mut query = Query::new(&self.symbol, &self.interval);
    query.set_all(&[Start(ms - step * self.len as i64), End(ms)]);

    let candles = API.read().unwrap().fetch_candles(&mut query)?;

    let mut val = 0.;
    for c in &candles {
      val += c.close;
    }
    val /= candles.len() as f32;
    self.val = val;

    let name = match self.exponential {
      true => "EMA",
      false => "MA",
    };
    log!("{} {} {} val: {}", self.interval, name, self.len, val);

    con().execute(
      "INSERT INTO moving_averages (symbol, interval, ms, len, val) values ($1, $2, $3, $4, $5)",
    &[&self.symbol, &self.interval, &self.ms, &self.len, &self.val]      
    )?;

    Ok(())
  }
}
