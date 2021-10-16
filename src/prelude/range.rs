use super::*;
use std::ops::Range;

pub trait MarketBombVecRange {
  fn num_candles(&self, ms: impl AsMs) -> usize;
}
impl MarketBombVecRange for Vec<Range<i64>> {
  fn num_candles(&self, step: impl AsMs) -> usize {
    let step = step.as_ms();
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
    let step = step.as_ms();
    self.start.round(step)..self.end.round(step)
  }
  fn num_candles(&self, step: impl AsMs) -> usize {
    let step = step.as_ms();
    ((self.end - self.start) / step) as usize
  }
  fn chunk(&self, step: impl AsMs, n: usize) -> Vec<Range<i64>> {
    let step = step.as_ms();
    let mut result = vec![];
    for i in (self.start..self.end).step_by(step as usize * n) {
      result.push(i..(i + step).min(self.end))
    }
    result
  }
}
