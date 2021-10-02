use crate::prelude::*;

pub fn create_moving_averages() {
  log!("Creating moving averages table.");
  con().batch_execute(
    "
CREATE TABLE moving_averages (
  ms           BIGINT NOT NULL,
  interval     VARCHAR(3) NOT NULL,
  len          INT NOT NULL,
  symbol       VARCHAR(10) NOT NULL,
  exponential  BOOLEAN NOT NULL,
  primary key  (ms, interval, len, symbol, exponential)
)",
  )
}
