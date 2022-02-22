use crate::prelude::*;
use anyhow::Result;

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
    _ => {
      log!("/yB Command not recognized.");
    }
  }
  Ok(())
}
