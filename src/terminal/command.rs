use crate::{data, prelude::*};
use anyhow::Result;

pub fn parse_command(cmd: String) -> Result<()> {
  let parts: Vec<&str> = cmd.split(" ").collect();

  if parts.is_empty() {
    return Ok(());
  }

  match parts[0] {
    "download" if parts.len() > 2 => {
      let ago_ms = ago_to_ms(parts[2])?;

      let api = Binance::new();

      let _ = api.fetch_candles("BTCUSDT", parts[1], ago_ms, now());
    }
    "calculate_meta" => {}
    _ => (),
  }
  Ok(())
}
