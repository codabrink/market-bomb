use crate::{database, prelude::*, Candle, CONFIG};
use anyhow::{anyhow, Result};
use std::{fs, path::Path, process::Command};

pub struct Frame<'a> {
  sp_values: Vec<(f32, f32, u8)>,
  detail_values: Vec<(f32, f32)>,
  sp_detail_delta: (f32, f32),
  pub dx_max: f32,
  pub dy_max: f32,
  pub ms: i64,
  symbol: &'a str,
  interval: &'a str,
  close: f32,
}

impl<'a> Frame<'a> {
  pub fn new(query: &mut Query, ms: i64) -> Result<Self> {
    let min_domain = CONFIG.strong_points.min_domain;
    let step = query.step();

    assert!(ms == round(ms, step));
    log!("Frame time: {}", ms_to_human(&ms));

    // First grab the detail candles (custom query)
    // Then grab strong points that are before (custom query)
    query.set_all(&[
      Start(ms - step * CONFIG.export.detail_view_len as i64),
      End(ms),
      Order(DESC),
    ]);
    let detail_candles = query.query_candles()?;

    assert!(detail_candles[0].open_time < detail_candles[1].open_time);
    // assert_eq!(detail_candles.len(), CONFIG.export.detail_view_len);
    assert_eq!(detail_candles.last().unwrap().open_time, ms);

    query.clear();
    query.set_all(&[
      BottomDomain(min_domain),
      TopDomain(min_domain),
      Limit(CONFIG.export.strong_point_length),
      Order(DESC),
      End(detail_candles[0].open_time - step),
    ]);
    let strong_point_candles = query.query_candles()?;

    let strong_points = strong_point::generate_points(&strong_point_candles);

    // They should not overlap
    assert!(
      detail_candles.first().unwrap().open_time
        != strong_point_candles.last().unwrap().open_time
    );

    // compile values
    let (mut dx_max, mut dy_max) = (0f32, 0f32);
    let detail_values = compile_detail_candles(&detail_candles, &mut dy_max);
    let sp_values =
      compile_strong_points(&strong_points, &mut dx_max, &mut dy_max);

    // calc distance between sp's and detail candles
    let last_sp = strong_points.last().unwrap();
    let first_dc = &detail_candles[0];
    let dx = first_dc.open_x() - last_sp.x;
    let dy = first_dc.open_y() - last_sp.y;
    let sp_detail_delta = (dx as f32, dy);

    log!(
      "=============================CLOSE: {}",
      detail_candles.last().unwrap().close
    );

    Ok(Frame {
      ms,
      detail_values,
      sp_values,
      dx_max,
      dy_max,
      symbol: query.symbol.clone(),
      interval: query.interval.clone(),
      close: detail_candles.last().unwrap().close,
      sp_detail_delta,
    })
  }

  pub fn pretty_time(&self) -> String {
    ms_to_human(&self.ms)
  }

  fn result(&self) -> Result<f32> {
    let mut con = con();
    let step = self.interval.to_step()?;
    let result_ms =
      self.ms + step * CONFIG.export.predict_candles_forward as i64;
    let candle = match con.query_candles(
      self.symbol,
      self.interval,
      Some(database::QueryOptions {
        start: Some(result_ms),
        limit: Some(1),
        ..Default::default()
      }),
    ) {
      Ok(candles) if candles.len() == 1 => candles[0],
      _ => return Err(anyhow!("No candle found at {}", self.ms)),
    };
    Ok((candle.open - self.close) as f32 / self.dy_max)
  }

  fn result_rounded(&self) -> Result<f32> {
    Ok(
      (match (self.result()? * 5.0).max(-6.0).min(6.0) {
        v if v < 0.5 && v > -0.5 => 0.0,
        v => v.round(),
      }) / 5.0,
    )
  }

  pub fn write_to_csv(&self, folder_path: &str) -> Result<()> {
    fs::create_dir_all(&folder_path).expect("Could not create directory");
    fs::write(
      format!("{}/{},{}.csv", &folder_path, self.ms, self.result()?),
      String::from(self),
    )
    .expect("Unable to write csv");
    Ok(())
  }

  pub fn write_to_csv_predict(&self) {
    let _ = fs::remove_dir_all("builder/csv/predict");
    fs::create_dir_all(format!("builder/csv/predict"))
      .expect("Could not create directory");
    fs::write(
      format!("builder/csv/predict/predict.csv"),
      String::from(self),
    )
    .expect("Unable to write csv");
  }

  pub fn predict(&self, candles_forward: usize) -> Result<()> {
    self.write_to_csv_predict();
    let _ = fs::remove_file("builder/prediction");

    let predict_cmd = format!(
      "python ./builder/predict.py {} {} {}",
      self.symbol, self.interval, candles_forward
    );
    Command::new("/bin/sh")
      .args(&["-c", predict_cmd.as_str()])
      .spawn()?
      .wait()?;

    while !Path::new("prediction").exists() {
      std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let ml_output: f32 = fs::read_to_string("prediction")?.parse()?;
    let _ = fs::remove_file("prediction");

    let step = self.interval.to_step()?;
    let mut prediction_time_ms = step * CONFIG.predict_candles_forward() as i64;
    prediction_time_ms += round(self.ms, step);
    let prediction_time_human = ms_to_human(&prediction_time_ms);

    let prediction_price = self.close + ml_output * self.dy_max;

    log!(
      "At {} the price will be {}",
      prediction_time_human,
      prediction_price
    );

    Ok(())
  }
}

fn compile_detail_candles(
  candles: &[Candle],
  dy_max: &mut f32,
) -> Vec<(f32, f32)> {
  candles
    .iter()
    .map(|c| {
      let dy = c.open - c.close;
      *dy_max = dy_max.max(dy);
      (c.close - c.open, c.wick_ratio())
    })
    .collect()
}
fn compile_strong_points(
  strong_points: &[StrongPoint],
  dx_max: &mut f32,
  dy_max: &mut f32,
) -> Vec<(f32, f32, u8)> {
  strong_points[1..]
    .iter()
    .enumerate()
    .map(|(i, sp)| {
      // Note that strong_points[i] != sp
      let dx = (sp.x - strong_points[i].x) as f32;
      let dy = sp.y - strong_points[i].y;
      *dx_max = dx_max.max(dx);
      *dy_max = dy_max.max(dy);
      (dx, dy, sp.position.into())
    })
    .collect()
}

impl<'a> From<&Frame<'a>> for String {
  fn from(frame: &Frame) -> Self {
    let mut result = String::new();
    let (x_ratio, y_ratio) = (1f32 / frame.dx_max, 1f32 / frame.dy_max);

    // Strong points
    result.push_str(
      &frame
        .sp_values
        .iter()
        .map(|v| format!("{},{},{},", v.0 * x_ratio, v.1 * y_ratio, v.2))
        .collect::<Vec<String>>()
        .join("\n"),
    );

    // dx and dy to open of first candle detail view
    let (dx, dy) = frame.sp_detail_delta;
    result.push_str(&format!("\n{},{},\n", dx * x_ratio, dy * y_ratio));

    // Detail candles
    result.push_str(
      &frame
        .detail_values
        .iter()
        .map(|(dy, wick_ratio)| format!("{},{},", dy * y_ratio, wick_ratio))
        .collect::<Vec<String>>()
        .join("\n"),
    );

    result.pop();

    result
  }
}

// Index is index of last/current candle
// candles and strongpoints are slices of a larger
//   candle/strongpoint vector
// Output structure is as such..

// integrate volume??

// -- STRONG POINTS (n of them)
// dx, dy, polarity, ema distances...
// dx, dy, polarity, ema distances...
// dx, dy, polarity, ema distances...
// ...
// -- DETAILED CANDLES (n of them)
// dx and dy to last strong_point
// dy, wick ratio
// dy, wick ratio
// dy, wick ratio
// ...

#[cfg(test)]
mod tests {
  use crate::prelude::*;
  use chrono::*;

  #[test]
  fn functional_frames() -> Result<()> {
    test_prep();

    let query = Query::new("BTCUSDT", "15m");
    let step = query.step();

    let start = to_ms(&Utc.ymd(2020, 01, 01).and_hms(0, 0, 0), step)?;
    let end = start + step * 100;

    query.set_all(&[Start(start), End(end)]);
    let missing = query.missing_candles()?;

    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].start, start);
    assert_eq!(missing[0].end, end - step); // ends in range are non-inclusive

    let api = Binance::new();
    let candles = api.fetch_candles(&mut query)?;

    assert_eq!(candles.len(), (start..end).num_candles(step) as usize);

    Ok(())
  }
}
