use crate::prelude::*;

pub struct Strategy {}

pub fn check_things() -> Result<()> {
  let mut q = Query::new("BTCUSDT", "4h");
  let candles = q.query_candles()?;
  // let ma =
  Ok(())
}
