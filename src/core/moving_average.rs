use crate::prelude::*;

pub struct MovingAverage {
  symbol: String,
  interval: String,
  ms: i64,
  len: i32,
  exponential: bool,
}

impl MovingAverage {
  fn save(&self) -> Result<()> {
    let step = self.interval.as_str().to_step()?;
    let ms = self.ms.round(step);

    let mut query = Query::new(&self.symbol, &self.interval);
    query.set_all(&[Start(ms - step * self.len as i64), End(ms)]);

    Ok(())
  }
}
