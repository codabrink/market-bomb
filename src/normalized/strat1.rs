use super::{Frame, StratStr};
use crate::prelude::*;

/// dp: delta-price
/// wm: wick-magnitude (ratio vs dp)
/// wpp: wick-percent-positive
/// ma: moving-average prices
pub struct Row {
  ms: i64,
  close: f32,
  // delta price
  dp: f32,
  // wick magnitude
  wm: f32,
  // wick percent positive
  wpp: f32,
  // moving averages
  ma: Vec<f32>,
}

// data exported from here is not normalized
pub fn export(strat: &str, symbol: &str) -> Result<Vec<Row>> {
  let start = (CONFIG.history_start as i64 + strat.strat_len()).round("1d");
  let end = now().round("1d");
  let train_end = (((end as f64 - start as f64) * 0.75) as i64).round("1d");

  let mut frames = strat.load(symbol, now())?;
  let mut result = convert(&mut frames)?;
  normalize(&mut result)?;

  let train_path =
    PathBuf::from(format!("builder/csv/{}/strat1/train", symbol));
  fs::create_dir_all(&train_path);
  let test_path = train_path.parent().unwrap().join("test");
  fs::create_dir_all(&test_path);

  Ok(result)
}

fn convert(frames: &mut Vec<Frame>) -> Result<Vec<Row>> {
  let mut result = Vec::with_capacity(frames.len());
  for i in 1..frames.len() {
    let f = frames[i];

    // body-low, body-high
    let bl = f.open.min(f.close);
    let bh = f.open.max(f.close);

    let wt = f.high - bh; // wick top: will be positive
    let wb = bl - f.low; // wick bottom: will be positive
    let wm = wt + wb; // wick magnitude

    let dp = f.close - frames[i - 1].close;

    result.push(Row {
      ms: f.ms,
      close: f.close,
      dp,
      wm,
      wpp: wt / wm,
      ma: f.ma,
    })
  }

  Ok(result)
}

/// normalize delta-price data on a scale from 1 to -1
/// normalize moving-average data to percent of price
fn normalize(rows: &mut Vec<Row>) -> Result<()> {
  let r = &rows[0];
  let max = rows.iter().fold(r.dp, |max, r| r.dp.max(max));

  for r in rows {
    for i in 0..r.ma.len() {
      r.ma[i] = r.ma[i] / r.close;
    }
    r.dp = r.dp / max;
    r.wm = r.wm / max;
  }

  Ok(())
}
