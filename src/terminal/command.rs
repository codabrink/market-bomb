use crate::prelude::*;
use anyhow::Result;
use normalized::*;

fn recognized() {
  log!("/gB Command recognized.");
}

pub fn parse_command(cmd: String) -> Result<()> {
  let parts: Vec<&str> = cmd.split(" ").collect();

  if parts.is_empty() {
    return Ok(());
  }

  match parts[0] {
    // download interval start(..end)
    "reset" => {
      recognized();
      con().batch_execute("delete from candles;")?;
      log!("Deleted all candles.");
    }
    "download" if parts.len() > 2 => {
      recognized();
      let range_parts: Vec<&str> = parts[2].split("..").collect();
      let start = range_parts[0].ago();
      let end = match range_parts.get(1) {
        Some(p) => p.ago(),
        _ => now(),
      };
      let mut query = Query::new("BTCUSDT", parts[1]);
      query.set_all(vec![Start(start), End(end)]);
      if start > end {
        bail!("Start of range must be before end.");
      }
      log!("Downloading candles from {} to {}.", start, end);

      let before_count = query.count_candles()?;
      let api = Binance::new();
      let _ = api.save_candles(&mut query)?;
      let after_count = query.count_candles()?;
      log!("Downloaded {} candles.", after_count - before_count);
    }
    "build_csv" => {
      recognized();

      let start = "30M".ago().round("12h".ms());
      let end = "1M".ago().round("12h".ms());

      fs::remove_dir_all("builder/csv")?;

      let train_path = PathBuf::from(format!("builder/csv/BTCUSDT/train.csv"));
      fs::create_dir_all(train_path.parent().unwrap())?;
      let mut train_file = File::create(&train_path)?;
      // export(start, end, &mut train_file)?;

      let test_path = PathBuf::from(format!("builder/csv/BTCUSDT/test.csv"));
      let mut test_file = File::create(&test_path)?;
      let start = "1M".ago().round("12h".ms());
      let end = "1d".ago().round("12h".ms());
      export(start, end, &mut test_file)?;
    }
    _ => {
      log!("/yB Command not recognized.");
    }
  }
  Ok(())
}

fn export(start: i64, end: i64, file: &mut File) -> Result<()> {
  let query = Query::new("BTCUSDT", "1h");
  let mut header = false;
  let strat1 = "52w:1w,6w:1d,1w:4h,4d:1h,2d:15m;4h:200:true,1d:50:false";

  for ms in (start..end).step_by("2h".ms() as usize) {
    match normalize(
      "BTCUSDT",
      ms,
      candle_segments.clone(),
      moving_averages.clone(),
    ) {
      Ok(d) => {
        let price_now = query.price(ms).unwrap();
        let price_future = query.price(ms + "24h".ms()).unwrap();

        let pct = (price_future - price_now) / price_now;

        if !header {
          header = true;
          let mut h = String::new();
          for i in 0..(d.len() * 6) {
            h.push_str(&format!("{},", i));
          }
          writeln!(file, "{}pct_change", h)?;
        }

        log!("Exporting: {}", ms.to_human());
        d.export(file, pct)?;
      }
      Err(e) => {
        log!("Export err: {:?}", e);
      }
    }
  }
  Ok(())
}
