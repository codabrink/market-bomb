use super::Candle;
// use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Serialize;
use std::ops::{Index, IndexMut};

#[derive(Default, Debug, Serialize)]
pub struct Candles {
  pub candles: Vec<Candle>,
  ms_width: i64,
  pub high: f32,
  pub low: f32,
  pub height: f32,
}

impl Candles {
  pub fn new(candles: Vec<Candle>) -> Self {
    let ms_width = candles.last().unwrap().open_time - candles[0].open_time;
    let (mut high, mut low) = (candles[0].high, candles[0].low);
    for candle in candles.iter() {
      if candle.low < low {
        low = candle.low;
      }
      if candle.high > high {
        high = candle.high;
      }
    }

    Candles {
      candles,
      ms_width,
      high,
      low,
      height: high - low,
    }
  }
  pub fn get(&self, index: usize) -> Option<&Candle> { self.candles.get(index) }
  pub fn len(&self) -> usize { self.candles.len() }
  pub fn first(&self) -> &Candle { &self.candles[0] }
  pub fn last(&self) -> &Candle { &self.candles[&self.candles.len() - 1] }
}

// impl Serialize for Candles {
// fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// where
// S: Serializer,
// {
// let mut seq = serializer.serialize_seq(Some(self.candles.len()))?;
// for c in self.candles {
// seq.serialize_element(c)?;
// }
// seq.end()
// }
// }

pub struct CandleIterator {
  candles: Candles,
  index: usize,
}

impl IntoIterator for Candles {
  type Item = Candle;
  type IntoIter = CandleIterator;
  fn into_iter(self) -> CandleIterator {
    CandleIterator {
      candles: self,
      index: 0,
    }
  }
}
impl Iterator for CandleIterator {
  type Item = Candle;
  fn next(&mut self) -> Option<Candle> {
    if self.index == self.candles.len() {
      return None;
    }
    self.index += 1;
    Some(self.candles[self.index - 1])
  }
}
impl Index<usize> for Candles {
  type Output = Candle;
  fn index(&self, i: usize) -> &Candle { &self.candles[i] }
}
impl IndexMut<usize> for Candles {
  fn index_mut(&mut self, i: usize) -> &mut Candle { &mut self.candles[i] }
}
