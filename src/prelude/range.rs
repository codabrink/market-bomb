use super::*;
pub use std::ops::Range;

pub trait MarketBombVecRange {
  fn num_candles(&self, ms: impl AsMs) -> usize;
}
impl MarketBombVecRange for Vec<Range<i64>> {
  fn num_candles(&self, step: impl AsMs) -> usize {
    let step = step.ms();
    self.iter().fold(0, |v, r| v + r.num_candles(step))
  }
}

pub trait MarketBombRange<T> {
  fn round(&self, step: impl AsMs) -> Self;
  fn num_candles(&self, step: impl AsMs) -> usize;
  fn chunk(&self, step: impl AsMs, n: usize) -> Vec<T>;
}

impl MarketBombRange<Range<i64>> for Range<i64> {
  fn round(&self, step: impl AsMs) -> Self {
    let step = step.ms();
    self.start.round(step)..self.end.round(step)
  }
  fn num_candles(&self, step: impl AsMs) -> usize {
    let step = step.ms();
    ((self.end - self.start) / step) as usize
  }
  fn chunk(&self, step: impl AsMs, n: usize) -> Vec<Range<i64>> {
    let step = step.ms();
    let mut result = vec![];
    for i in (self.start..self.end).step_by(step as usize * n) {
      result.push(i..(i + step).min(self.end))
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use chrono::{TimeZone, Utc};

  use crate::prelude::*;

  #[test]
  fn range() -> Result<()> {
    let noon = Utc.ymd(2020, 1, 1).and_hms(12, 0, 0).ms();
    let step = "15m".ms();
    let hour = "1h".ms();

    // there are four fifteen minute candles in an hour
    assert_eq!((noon..(noon + hour)).num_candles(step), 4);

    Ok(())
  }
}
