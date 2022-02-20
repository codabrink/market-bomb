use crate::prelude::*;

pub fn build_cache(symbol: &str) -> Result<()> {
  let config = Config::load();
  let mut q = Query::new(symbol, "1d");

  let history_start = format!("{}d", config.history_start).ago();
  let history_end = format!("{}d", config.history_end).ago();

  for interval in ["1d", "4h", "1h"] {
    q.set_interval(interval);
    q.set_all(&[Start(history_start), End(history_end)]);
    API.save_candles(&mut q)?;
  }

  q.set_interval("15m");
  q.set_all(&[Start(history_end - "1y".ms()), End(history_end)]);
  API.save_candles(&mut q)?;

  MovingAverage::calculate_ema(symbol, "4h", 200)?;

  Ok(())
}
