use crate::{api::*, chart::Candle, data::*};
use anyhow::Result;
use std::{ops::Range, process::Command, sync::RwLock};

lazy_static! {
  pub static ref POOL: DbPool = init_pool();
}
pub enum Order {
  ASC,
  DESC,
}

#[derive(Default)]
pub struct QueryOptions {
  pub start: Option<i64>,
  pub end: Option<i64>,
  pub limit: Option<usize>,
  pub order: Option<Order>,
  pub top_domain: Option<i32>,
  pub bottom_domain: Option<i32>,
}
lazy_static! {
  pub static ref DATABASE: RwLock<String> = RwLock::new("trader".into());
}

pub fn test() {
  *DATABASE.write().unwrap() = String::from("trader_test");
  let _ = con().batch_execute("DELETE FROM candles;");
}

pub trait MyDbCon {
  fn query_candles(
    &mut self,
    symbol: &str,
    interval: &str,
    options: Option<QueryOptions>,
  ) -> Result<Vec<Candle>>;
  fn missing_candles(
    &mut self,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Result<Vec<Range<i64>>>;
  fn insert_candle(
    &mut self,
    symbol: &str,
    interval: &str,
    candle: &Candle,
  ) -> Result<()>;
  fn copy_in_candles(
    &mut self,
    out: String,
    symbol: &str,
    interval: &str,
  ) -> Result<()>;
}

impl MyDbCon for DbCon {
  fn query_candles(
    &mut self,
    symbol: &str,
    interval: &str,
    options: Option<QueryOptions>,
  ) -> Result<Vec<Candle>> {
    let options = match options {
      Some(o) => o,
      None => QueryOptions {
        ..Default::default()
      },
    };

    let mut query = format!(
      "SELECT {} FROM candles WHERE symbol = $1 AND interval = $2 AND dead = false",
      Candle::DB_COLUMNS
    );
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&symbol, &interval];
    let mut i = 2;
    if let Some(start) = &options.start {
      i += 1;
      query.push_str(format!(" AND open_time >= ${}", i).as_str());
      params.push(start);
    }
    if let Some(end) = &options.end {
      i += 1;
      query.push_str(format!(" AND open_time <= ${}", i).as_str());
      params.push(end);
    }
    if let Some(top_domain) = &options.top_domain {
      i += 1;
      query.push_str(format!(" AND top_domain >= ${}", i).as_str());
      params.push(top_domain);
    }
    if let Some(bottom_domain) = &options.bottom_domain {
      i += 1;
      query.push_str(format!(" AND bottom_domain >= ${}", i).as_str());
      params.push(bottom_domain);
    }
    match options.order {
      Some(Order::DESC) => query.push_str(" ORDER BY open_time DESC"),
      _ => query.push_str(" ORDER BY open_time ASC"),
    }
    let limit = match options.limit {
      Some(l) => l,
      None => CONFIG.query_limit,
    };
    query.push_str(format!(" LIMIT {}", limit).as_str());

    // println!("{}", query.as_str());
    // println!("{:?}", &params);
    let rows = self.query(query.as_str(), &params)?;
    Ok(rows.iter().enumerate().map(Candle::from).collect())
  }
  fn missing_candles(
    &mut self,
    symbol: &str,
    interval: &str,
    start: i64,
    end: i64,
  ) -> Result<Vec<Range<i64>>> {
    let step = interval.to_step()?;
    let start = round(start, step);
    let end = round(end, step) - step;

    let rows = self
          .query(
              "
  SELECT c.open_time AS missing_open_times
  FROM generate_series($1::bigint, $2::bigint, $3::bigint) c(open_time)
  WHERE NOT EXISTS (SELECT 1 FROM candles where open_time = c.open_time AND symbol = $4 AND interval = $5);",
              &[&start, &end, &step, &symbol, &interval],
          )
          .unwrap();

    let missing: Vec<i64> = rows.iter().map(|i| i.get(0)).collect();

    let mut range_start = 0;
    let mut result = Vec::<Range<i64>>::new();

    // group the missing candles
    for i in 1..missing.len() {
      if missing[i - 1] + step != missing[i] {
        result.push(missing[range_start]..missing[i - 1]);
        range_start = i;
      }
    }
    // push the leftovers
    if missing.len() > range_start {
      result.push(missing[range_start]..missing[missing.len() - 1]);
    }
    Ok(result)
  }
  fn copy_in_candles(
    &mut self,
    out: String,
    symbol: &str,
    interval: &str,
  ) -> Result<()> {
    fs::create_dir_all("/tmp/pg_copy")?;
    let header = "id, symbol, interval, open_time, open, high, low, close, volume, close_time, bottom_domain, top_domain, fuzzy_domain, dead, indicators";
    let mut _out = String::from(format!("{}\n", header));
    _out.push_str(out.as_str());

    let p = format!("/tmp/pg_copy/{}-{}.csv", symbol, interval);
    let path = Path::new(&p).to_str().unwrap();
    fs::write(path, out).unwrap();
    self.batch_execute("delete from import_candles;").unwrap();
    Command::new("psql")
      .arg("-d")
      .arg(db())
      .arg("-c")
      .arg(&*format!(
        r#"\copy import_candles({header}) FROM '{csv_path}' CSV DELIMITER E'\t' QUOTE '"' ESCAPE '\';"#,
        header = header, csv_path = path
      ))
      .output()
      .expect("Failed to copy in candles");

    self
      .batch_execute(
        format!(
          "
  DELETE FROM candles WHERE
  open_time IN (SELECT open_time FROM import_candles)
  AND symbol = '{symbol}' AND interval = '{interval}';
  INSERT INTO candles SELECT * FROM import_candles;
  ",
          symbol = symbol,
          interval = interval
        )
        .as_str(),
      )
      .unwrap();

    Ok(())
  }

  fn insert_candle(
    &mut self,
    symbol: &str,
    interval: &str,
    candle: &Candle,
  ) -> Result<()> {
    self
      .execute(
        "
  INSERT INTO candles (
    symbol,
    interval,
    open_time,
    close_time,
    open,
    high,
    low,
    close,
    volume,
    dead,
    indicators,
    source
  ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        &[
          &symbol,
          &interval,
          &(candle.open_time as i64),
          &(candle.close_time as i64),
          &candle.open,
          &candle.high,
          &candle.low,
          &candle.close,
          &candle.volume,
          &candle.dead,
          &serde_json::to_string(&candle.indicators)?,
          &"binance",
        ],
      )
      .ok();

    Ok(())
  }
}

fn db() -> String { DATABASE.read().unwrap().clone() }

fn init_pool() -> DbPool {
  if !database_exists() {
    println!("Database '{}' not found", db());
    create_db().expect("Could not create database");
    migrate().expect("Could not migrate database");
  }
  let manager = r2d2_postgres::PostgresConnectionManager::new(
    format!("host=127.0.0.1 user=postgres dbname={}", db())
      .parse()
      .unwrap(),
    NoTls,
  );
  Pool::new(manager).unwrap()
}
pub fn con() -> DbCon { POOL.clone().get().unwrap() }

pub fn database_exists() -> bool {
  let a = Command::new("psql")
    .arg("-tAc")
    .arg(format!(
      r#"SELECT 1 FROM pg_database WHERE datname='{}'"#,
      db()
    ))
    .output()
    .unwrap();
  String::from_utf8_lossy(&a.stdout).trim().eq("1")
}
pub fn reset() { con().batch_execute("DELETE FROM candles;").unwrap(); }
pub fn create_db() -> Result<()> {
  print!("Creating database...");
  Client::connect("host=127.0.0.1 user=postgres", NoTls)?
    .batch_execute(format!("CREATE DATABASE {};", db()).as_str())?;
  println!("Done");
  Ok(())
}

fn migrate() -> Result<()> {
  print!("Migrating database...");
  Client::connect(
    format!("host=127.0.0.1 user=postgres dbname={}", db()).as_str(),
    NoTls,
  )?
  .batch_execute(
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
  ma_20         REAL,
  ma_50         REAL,
  ma_200        REAL,
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
  println!("Done");
  Ok(())
}

pub fn update_domain(con: &mut DbCon, id: &i32, (top, bottom): (i32, i32)) {
  con
    .execute(
      "UPDATE candles SET top_domain = $2, bottom_domain = $3 WHERE id = $1",
      &[&id, &top, &bottom],
    )
    .unwrap();
}

fn ma_calc_query(len: usize) -> String {
  format!(
    "
  UPDATE candles as a
    SET ma_{len} = SUM(
      SELECT b.close FROM candles as b
      WHERE b.open_time < a.open_time AND
      b.symbol = a.symbol AND
      b.interval = a.interval
      ORDER BY b.open_time DESC
      LIMIT {len}
    ) / {len};",
    len = len
  )
}

pub fn calculate_all_ma(con: &mut DbCon, interval: &str) -> Result<()> {
  for len in [20, 50, 200] {
    log!("Calculating {} ma..", len);
    con.batch_execute(&ma_calc_query(len))?;
  }

  Ok(())
}

pub fn calculate_domain(con: &mut DbCon, interval: &str) -> Result<()> {
  println!("Calculating domain for... {}", interval);
  let step = interval.to_step()?;
  con.execute(
    "
UPDATE candles AS a
  SET top_domain = (
    COALESCE(ABS(
      (SELECT b.open_time FROM candles AS b
      WHERE b.high >= a.high AND
      b.open_time != a.open_time AND
      b.symbol = a.symbol AND
      b.interval = $2
      ORDER BY ABS(a.open_time - b.open_time)
      LIMIT 1) - a.open_time
    ) / $1, 0)
  ),
  bottom_domain = (
    COALESCE(ABS(
      (SELECT b.open_time FROM candles as b
      WHERE b.low <= a.low AND
      b.open_time != a.open_time AND
      b.symbol = a.symbol AND
      b.interval = $2
      ORDER BY ABS(a.open_time - b.open_time)
      LIMIT 1) - a.open_time
    ) / $1, 0)
  )
  WHERE a.interval = $2
  AND (a.top_domain = 0 OR a.bottom_domain = 0);",
    &[&(step as i64), &interval],
  )?;
  Ok(())
}

// (top, bottom)
pub fn _breaking_candles(
  con: &mut DbCon,
  open_time: i64,
  symbol: &str,
  interval: &str,
  high: Option<f32>,
  low: Option<f32>,
) -> Result<(Option<Vec<i64>>, Option<Vec<i64>>)> {
  let day_step = "1d".to_step()?;
  let day_ms = round(open_time, day_step);

  // if days are equally distant, return both. If not, return closest.
  let clean_nearest = |n: Vec<i64>| match n.len() {
    2 => match (day_ms - n[0]).abs() - (day_ms - n[1]).abs() {
      d if d > 0 => vec![n[1]],
      d if d < 0 => vec![n[0]],
      _ => n,
    },
    _ => n,
  };
  let mut query_nearest = |q: &str, v: &f32| {
    let rows = con
            .query(
                &*format!(
                    "SELECT open_time FROM candles where {} AND symbol = $2 AND interval = $3 ORDER BY ABS(open_time - $4) LIMIT 2",
                    q
                ),
                &[&v, &symbol, &interval, &(open_time as i64)],
            )
            .unwrap();
    Some(clean_nearest(rows.iter().map(|r| r.get(0)).collect()))
  };

  let top = match high {
    Some(high) => query_nearest("where high >= $1", &high),
    None => None,
  };
  let bottom = match low {
    Some(low) => query_nearest("where low <= $1", &low),
    None => None,
  };
  Ok((top, bottom))
}
