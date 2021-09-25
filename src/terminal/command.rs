use crate::prelude::*;
use anyhow::Result;

pub fn parse_command(cmd: String) -> Result<()> {
  let parts: Vec<&str> = cmd.split(" ").collect();

  if parts.is_empty() {
    return Ok(());
  }

  match parts[0] {
    // download interval start(..end)
    "download" if parts.len() > 2 => {
      log!("Command recognized.");
      let range_parts: Vec<&str> = parts[2].split("..").collect();
      let start = ago_to_ms(range_parts[0])?;
      let end = match range_parts.get(1) {
        Some(p) => ago_to_ms(p)?,
        _ => now(),
      };
      if start > end {
        bail!("Start of range must be before end.");
      }
      log!("Downloading candles from {} to {}.", start, end);

      let api = Binance::new();
      let _ = api.fetch_candles("BTCUSDT", parts[1], start, end);
    }
    "calculate_meta" => {}
    _ => (),
  }
  Ok(())
}
