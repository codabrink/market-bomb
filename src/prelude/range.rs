use super::*;
use std::ops::Range;

pub trait MarketBombRange<T> {
  fn round(&self, step: &str) -> Self;
  fn round_ms(&self, step: i64) -> Self;
  fn num_candles(&self, step: i64) -> usize;
  fn chunk(&self, step: i64, n: usize) -> Vec<T>;
}

impl MarketBombRange<Range<i64>> for Range<i64> {
  fn round(&self, step: &str) -> Self {
    let step = step.to_step().unwrap();
    self.round_ms(step)
  }
  fn round_ms(&self, step: i64) -> Self {
    self.start.round(step)..self.end.round(step)
  }
  fn num_candles(&self, step: i64) -> usize {
    ((self.end - self.start) / step) as usize
  }
  fn chunk(&self, step: i64, n: usize) -> Vec<Range<i64>> {
    let mut result = vec![];
    for i in (self.start..self.end).step_by(step as usize * n) {
      result.push(i..(i + step).min(self.end))
    }
    result
  }
}
