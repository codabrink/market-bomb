use crate::chart::{Candle, Candles, Chart, LineCross};

use rayon::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use CandlePos::*;

use super::CandlePos;

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct TrendLineMeta {
  pub before_check: u64,
  pub after_check: u64,
  pub min_width: u64,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct TrendLine {
  pub line: Line,
  pub angle: f32,
  pub crosses: Vec<LineCross>,
  pub strength: f32,
  pub p1: (f32, f32),
  pub p2: (f32, f32),
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct LineRoot {
  pub candle_index: usize,
  pub candle_position: CandlePos,
  pub x: i64,
  pub y: f32,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Line {
  pub b: f32,
  pub slope: f32,
  pub roots: Vec<LineRoot>,
}

impl TrendLine {
  pub fn new_from_candle_indexes(
    candles: Arc<Candles>,
    i1: usize,
    i2: usize,
    pos: CandlePos,
  ) -> Option<TrendLine> {
    let (c1, c2) = match (candles.get(i1), candles.get(i2)) {
      (Some(c1), Some(c2)) => (c1, c2),
      _ => return None,
    };

    let ((x1, y1), (x2, y2)) = match pos {
      HIGH => ((c1.open_time, c1.high), (c2.open_time, c2.high)),
      LOW => ((c1.open_time, c1.low), (c2.open_time, c2.low)),
    };
    let line = Line::new(candles.clone(), i1, pos, i2, pos);

    let max_x = candles.last().open_time;
    let max_y = line.y_at_x(max_x as f32);

    let trend_line = TrendLine {
      line,
      crosses: vec![],
      strength: 0f32,
      angle: 0f32,
      p1: (x1 as f32, y1),
      p2: (max_x as f32, max_y),
    };

    // TODO: add crosses

    Some(trend_line)
  }

  pub fn check_backwards(
    &self,
    chart: Arc<Chart>,
    extra_candles_reverse: Arc<Vec<Candle>>,
  ) -> bool {
    let mut index = self.line.roots[0].candle_index;
    let (c1, c2) = (
      chart.candles[self.line.roots[0].candle_index],
      chart.candles[self.line.roots[1].candle_index],
    );
    let min_open_time = c1.open_time - (c2.open_time - c1.open_time);

    while let Some(c) = chart.candles.get(index) {
      index -= 1;
      if self.line.intersects(c) {
        return false;
      }
      if index == 0 || c.open_time <= min_open_time {
        break;
      }
    }
    index = 0;
    while let Some(c) = extra_candles_reverse.get(index) {
      if c.open_time <= min_open_time {
        break;
      }
      if self.line.intersects(c) {
        return false;
      }
      index += 1;
    }

    true
  }
}

impl Line {
  pub fn new(
    candles: Arc<Candles>,
    ci1: usize,
    cp1: CandlePos,
    ci2: usize,
    cp2: CandlePos,
  ) -> Line {
    let (c1, c2) = (&candles[ci1], &candles[ci2]);
    let (p1x, p1y) = (
      c1.open_time,
      match cp1 {
        HIGH => c1.high,
        LOW => c1.low,
      },
    );
    let (p2x, p2y) = (
      c2.open_time,
      match cp2 {
        HIGH => c2.high,
        LOW => c2.low,
      },
    );

    let mut line = Line {
      b: 0f32,
      slope: 0f32,
      roots: vec![
        LineRoot {
          x: p1x,
          y: p1y,
          candle_index: ci1,
          candle_position: cp1,
        },
        LineRoot {
          x: p2x,
          y: p2y,
          candle_index: ci2,
          candle_position: cp2,
        },
      ],
    };
    line.recalculate();
    line
  }

  pub fn update_root(
    &mut self,
    i: usize,
    candles: Arc<Candles>,
    ci: usize,
    cp: CandlePos,
  ) {
    let c = &candles[ci];
    self.roots[i] = LineRoot {
      x: c.open_time,
      y: match cp {
        HIGH => c.high,
        LOW => c.low,
      },
      candle_index: ci,
      candle_position: cp,
    };
    self.recalculate();
  }

  pub fn recalculate(&mut self) {
    self.slope = (self.roots[1].y - self.roots[0].y)
      / (self.roots[1].x - self.roots[0].x) as f32;
    self.b = self.roots[0].y - self.slope * self.roots[0].x as f32;
  }

  pub fn width(&self) -> usize {
    self.roots[1].candle_index - self.roots[0].candle_index
  }

  // distance from line to candle
  // if line is above, result is negative. if line is below, result is positive.
  pub fn vertical_distance(&self, candle: &Candle, body_only: bool) -> f32 {
    let y = self.y_at_x(candle.open_time as f32);
    let (high, low) = match body_only {
      true => (candle.open.max(candle.close), candle.open.min(candle.close)),
      false => (candle.high, candle.low),
    };
    match (y, high, low) {
      (y, high, low) if y >= low && y <= high => 0f32,
      (y, high, _) if y > high => high - y,
      (y, _, low) => low - y,
    }
  }

  pub fn intersects(&self, candle: &Candle) -> bool {
    let y = self.y_at_x(candle.open_time as f32);
    y > candle.low && y < candle.high
  }

  pub fn y_at_x(&self, x: f32) -> f32 { (self.slope * x) + self.b }
}

pub fn generate_bounding_lines(candles: &Arc<Candles>) -> Vec<TrendLine> {
  let mut all_lines = Vec::<Line>::new();
  let mut index = 0;
  while index < candles.len() {
    let mut lines = crawl(&candles, index, LOW);
    index = match lines.len() {
      0 => index + 1,
      _ => lines.last().unwrap().roots[1].candle_index,
    };
    all_lines.append(&mut lines);
  }

  index = 0;
  while index < candles.len() {
    let mut lines = crawl(&candles, index, HIGH);
    index = match lines.len() {
      0 => index + 1,
      _ => lines[0].roots[1].candle_index,
    };
    all_lines.append(&mut lines);
  }

  let res: Vec<TrendLine> = all_lines
    .par_iter()
    .map(|l| {
      TrendLine::new_from_candle_indexes(
        candles.clone(),
        l.roots[0].candle_index,
        l.roots[1].candle_index,
        l.roots[0].candle_position,
      )
    })
    .filter_map(|l| l)
    .collect();

  res
}

enum Dir {
  FRWD,
  BACK,
}

const MIN_LIFT: f32 = 0.03;

fn crawl(
  candles: &Arc<Candles>,
  mut index: usize,
  pos: CandlePos,
) -> Vec<Line> {
  let min_width = 4usize;
  let before_check = 4usize;
  let after_check = 4usize;

  let mut lines = vec![];
  if index + min_width > candles.len() - 1 {
    return lines;
  }

  let min_lift = candles.height * MIN_LIFT;

  let mut lift_sustain = 0;
  let (c1, c2) = (&candles[index], &candles[index + 1]);
  let mut line = Line::new(candles.clone(), index, pos, index + 1, pos);
  let mut current_line_added = false;

  for i in (index + 1)..(candles.len() - 1) {
    let candle = &candles[i];
    let y = line.y_at_x(candle.open_time as f32);
    let pos = line.roots[1].candle_position;
    match (y, candle.high, candle.low) {
      // is the line on the wrong side of, or in the candle?
      (y, h, l) if pos == HIGH && y <= h || pos == LOW && y >= l => {
        line.update_root(1, candles.clone(), i, pos);
        current_line_added = false;
        lift_sustain = 0;
      }
      // is the line far enough away to be considered 'lift_sustain'?
      (y, h, l)
        if pos == HIGH && y - h > min_lift
          || pos == LOW && l - y > min_lift =>
      {
        lift_sustain += 1
      }
      // assume the candle is above the line, but not high enough
      _ => (),
    }
    if !current_line_added
      && line.width() >= min_width
      && lift_sustain > after_check
    {
      // we've moved away from the line for long and far enough; let's make the line.
      if add(candles, &mut lines, &mut line) {
        current_line_added = true;
        lift_sustain = 0;
      }
    }
    index += 1;
  }
  // if line.width() >= 10 && line.roots[1].candle_index < chart.candles.len() - 5 {
  // lines.push(line);
  // }
  lines
}

fn add(candles: &Arc<Candles>, lines: &mut Vec<Line>, line: &mut Line) -> bool {
  use std::cmp::max;

  let before_check = 4usize;
  let after_check = 4usize;

  let i1 = line.roots[0].candle_index;
  let pos = line.roots[0].candle_position;
  if let Some(l) = lines.last() {
    if pos == LOW && line.slope >= l.slope
      || pos == HIGH && line.slope <= l.slope
    {
      return false;
    }
  }

  // check behind
  for i in (max(i1, before_check) - before_check)..i1 {
    let c = &candles[i];
    let y = line.y_at_x(c.open_time as f32);
    let pos = line.roots[0].candle_position;

    match (pos, y, c.high, c.low) {
      (LOW, y, _, l) if y >= l => return false,
      (HIGH, y, h, _) if y <= h => return false,
      _ => (),
    }
  }
  lines.push(line.clone());
  true
}
