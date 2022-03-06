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
    "predict" => {
      recognized();
      let data1 = "52w:1w,6w:1d,1w:4h,4d:1h,2d:15m;4h:200:true,1d:50:false";

      let mut file = File::open("predict.csv")?;
      normalized::strat1::export_at(data1, "BTCUSDT", now(), &mut file)?;
    }
    "build_csv" => {
      recognized();

      let data1 = "52w:1w,6w:1d,1w:4h,4d:1h,2d:15m;4h:200:true,1d:50:false";
      normalized::strat1::export_all(data1, "BTCUSDT")?;
    }
    _ => {
      log!("/yB Command not recognized.");
    }
  }
  Ok(())
}
