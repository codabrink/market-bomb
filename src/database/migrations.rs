use crate::prelude::*;

fn con() -> postgres::Client {
  Client::connect(
    format!("host=127.0.0.1 user=postgres dbname={}", super::db()).as_str(),
    NoTls,
  )
  .unwrap()
}

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
  bottom_domain INT DEFAULT 0 NOT NULL,
  top_domain    INT DEFAULT 0 NOT NULL,
  fuzzy_domain  BOOLEAN DEFAULT TRUE,
  derived       BOOLEAN DEFAULT FALSE,
  source        TEXT NOT NULL,
  primary key   (open_time, interval, symbol, source)
);
CREATE INDEX derived_idx ON candles (derived);
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
  exp          BOOLEAN NOT NULL,
  val          REAL NOT NULL,
  primary key  (ms, interval, len, symbol, exp)
)",
  )?;
  Ok(())
}
