use super::{Frame, StratStr};
use crate::prelude::*;

/// dp: delta-price
/// wm: wick-magnitude (ratio vs dp)
/// wpp: wick-percent-positive
/// ma: moving-average prices
pub struct Row {
  ms: i64,
  // close price (not normalized)
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

trait WriteableRows {
  fn write(&self, file: &mut File, label: f32) -> Result<()>;
}
impl WriteableRows for Vec<Row> {
  fn write(&self, file: &mut File, label: f32) -> Result<()> {
    for row in self {
      let ma = row
        .ma
        .iter()
        .map(|ma| ma.to_string())
        .collect::<Vec<String>>()
        .join(",");
      writeln!(file, "{},{},{},{}", row.dp, row.wm, row.wpp, ma)?;
    }
    write!(file, "{}", label)?;
    Ok(())
  }
}

// data exported from here is not normalized
pub fn export(strat: &str, symbol: &str) -> Result<()> {
  let start = (now() - format!("{}d", CONFIG.history_start).ms()
    + strat.strat_len())
  .round("1d");
  let end = (now() - "2d".ms()).round("1d");
  let train_end =
    (((end as f64 - start as f64) * 0.75) as i64 + start).round("1d");

  let train_path =
    PathBuf::from(format!("builder/csv/{}/strat1/train", symbol));
  let _ = fs::remove_dir_all(train_path.parent().unwrap());
  fs::create_dir_all(&train_path)?;
  let test_path = train_path.parent().unwrap().join("test");
  fs::create_dir_all(&test_path)?;

  log!(
    "Start: {}, Training End: {}, End: {}",
    start.to_human(),
    train_end.to_human(),
    end.to_human()
  );

  let pb_label = "Exporting train CSV...".to_string();
  let _ = terminal::PB.0.send((pb_label.clone(), 0.));
  for cursor in (start..train_end).step_by("1w".ms() as usize) {
    let pct = (cursor - start) as f64 / (train_end - start) as f64;
    let _ = terminal::PB.0.send((pb_label.clone(), pct));

    let frames = match strat.load(symbol, cursor) {
      Ok(f) => f,
      Err(e) => {
        log!("Error: {:?}", e);
        continue;
      }
    };
    let mut result = convert(&frames)?;
    normalize(&mut result)?;

    let p = train_path.join(format!("{}.csv", cursor));
    let mut file = File::create(&p)?;

    let q = Query::new(symbol, "15m");
    let label = q.price(cursor + "8h".ms()).expect("Could not get price.");

    let close = frames.last().unwrap().close;
    let label = (label - close) / close;

    result.write(&mut file, label)?;
  }
  let _ = terminal::PB.0.send((pb_label.clone(), -1.));

  let pb_label = "Exporting test CSV...".to_string();
  let _ = terminal::PB.0.send((pb_label.clone(), 0.));
  for cursor in (train_end..end).step_by("1w".ms() as usize) {
    let pct = (cursor - train_end) as f64 / (end - train_end) as f64;
    let _ = terminal::PB.0.send((pb_label.clone(), pct));

    let frames = strat.load(symbol, cursor)?;
    let mut result = convert(&frames)?;
    normalize(&mut result)?;

    let p = test_path.join(format!("{}.csv", cursor));
    let mut file = File::create(&p)?;

    let q = Query::new(symbol, "15m");
    let label = q.price(cursor + "8h".ms()).expect("Could not get price.");

    let close = frames.last().unwrap().close;
    let label = (label - close) / close;

    result.write(&mut file, label)?;
  }
  let _ = terminal::PB.0.send((pb_label.clone(), -1.));

  Ok(())
}

fn convert(frames: &Vec<Frame>) -> Result<Vec<Row>> {
  let mut result = Vec::with_capacity(frames.len());
  for i in 1..frames.len() {
    let f = &frames[i];

    // body-low, body-high
    let bl = f.open.min(f.close);
    let bh = f.open.max(f.close);

    let wt = f.high - bh; // wick top: will be positive
    let wb = bl - f.low; // wick bottom: will be positive
    let wm = wt + wb; // wick magnitude

    // TODO: risk of divide by 0.
    let mut wpp = wt / wm;
    if wpp.is_nan() {
      wpp = 0.;
    }

    let dp = f.close - frames[i - 1].close;

    result.push(Row {
      ms: f.ms,
      close: f.close,
      dp,
      wm,
      wpp,
      ma: f.ma.clone(),
    })
  }

  Ok(result)
}

/// normalize delta-price data on a scale from 1 to -1
/// normalize moving-average data to percent of price
fn normalize(rows: &mut Vec<Row>) -> Result<()> {
  let r = &rows[0];
  let max = rows.iter().fold(r.dp.abs(), |max, r| r.dp.max(max.abs()));

  for r in rows {
    for i in 0..r.ma.len() {
      r.ma[i] = r.ma[i] / r.close;
    }
    r.dp = r.dp / max;
    r.wm = r.wm / max;
  }

  Ok(())
}
