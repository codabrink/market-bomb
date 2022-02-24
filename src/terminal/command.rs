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
      let mut query = Query::new("BTCUSDT", "1h");
      for ms in "1y".ago().."2y".ago() {
        let (max, min, d) = normalize(
          "BTCUSDT",
          ms,
          vec![
            CandleSegment::new("1y", "1w"),
            CandleSegment::new("6w", "1d"),
            CandleSegment::new("1w", "4h"),
            CandleSegment::new("4d", "1h"),
            CandleSegment::new("2d", "15m"),
          ],
          vec![
            MA {
              interval: "4h".to_string(),
              len: 200,
              exp: true,
            },
            MA {
              interval: "1d".to_string(),
              len: 50,
              exp: false,
            },
          ],
        )?;

        let price_now = query.price(ms).unwrap();
        let price_future = query.price(ms + "8h".ms()).unwrap();

        let pct = (price_future - price_now) / price_now;
        let p = PathBuf::from(format!("builder/csv/train/BTCUSDT/{}.csv", pct));
        d.export(&p)?;
      }
    }
    _ => {
      log!("/yB Command not recognized.");
    }
  }
  Ok(())
}
