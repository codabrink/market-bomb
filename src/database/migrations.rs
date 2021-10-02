use crate::prelude::*;

pub fn create_candles_table() -> Result<()> {
  con().batch_execute(
    "
CREATE TABLE candles (
  id            SERIAL,
  interval      VARCHAR(3) NOT NULL,
  symbol        VARCHAR(10) NOT NULL,
  open_time     BIGINT NOT NULL,
  close_time    BIGINT NOT NULL,
  open          REAL NOT NULL,
  high          REAL NOT NULL,
  low           REAL NOT NULL,
  close         REAL NOT NULL,
  volume        REAL NOT NULL,
  indicators    TEXT NOT NULL,
  bottom_domain INT DEFAULT 0 NOT NULL,
  top_domain    INT DEFAULT 0 NOT NULL,
  fuzzy_domain  BOOLEAN DEFAULT TRUE,
  dead          BOOLEAN DEFAULT FALSE,
  source        TEXT NOT NULL,
  primary key   (open_time, interval, symbol, dead, source)
);
CREATE TABLE import_candles AS TABLE candles WITH NO DATA;",
  )?;
  Ok(())
}

pub fn create_moving_averages_table() -> Result<()> {
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
  )?;
  Ok(())
}
