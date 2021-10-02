use crate::prelude::*;
use anyhow::Result;
use std::{
  mem::{discriminant, Discriminant},
  ops::Range,
  process::Command,
  sync::RwLock,
};

lazy_static! {
  pub static ref POOL: DbPool = init_pool();
}
#[derive(Clone)]
pub enum Order {
  ASC,
  DESC,
}

lazy_static! {
  pub static ref DATABASE: RwLock<String> = RwLock::new("trader".into());
}

pub fn test() {
  *DATABASE.write().unwrap() = String::from("trader_test");
  let _ = con().batch_execute("DELETE FROM candles;");
}

pub struct Query<'a> {
  pub con: DbCon,
  pub symbol: &'a str,
  pub interval: &'a str,
  options: AHashMap<Discriminant<QueryOpt>, QueryOpt>,
}

#[derive(Clone)]
pub enum QueryOpt {
  Start(i64),
  End(i64),
  Limit(usize),
  Order(Order),
  TopDomain(i32),
  BottomDomain(i32),
}
impl<'a> Query<'a> {
  pub fn new(symbol: &'a str, interval: &'a str) -> Self {
    Self {
      symbol,
      interval,
      con: con(),
      options: AHashMap::new(),
    }
  }
  pub fn get(&self, opt: &QueryOpt) -> Option<&QueryOpt> {
    self.options.get(&discriminant(opt))
  }
  pub fn set(&mut self, opt: QueryOpt) {
    self.options.insert(discriminant(&opt), opt);
  }
  pub fn set_all(&mut self, opt: &[QueryOpt]) {
    for opt in opt {
      self.set(opt.clone());
      // self.options.insert(discriminant(opt), opt.clone());
    }
  }
  pub fn is_empty(&self) -> bool { self.num_candles() == 0 }
  pub fn num_candles(&self) -> usize {
    match (self.get(&Start(0)), self.get(&End(0))) {
      (Some(Start(start)), Some(End(end))) if start > end => {
        (*start..*end).num_candles(self.step())
      }
      _ => 0,
    }
  }
  pub fn clear(&mut self) { self.options.clear(); }
  pub fn remove(&'a mut self, opt: &QueryOpt) {
    self.options.remove(&discriminant(&opt));
  }
  pub fn range(&self) -> Option<Range<i64>> {
    match (self.get(&Start(0)), self.get(&End(0))) {
      (Some(Start(start)), Some(End(end))) => Some(*start..*end),
      _ => None,
    }
  }
  pub fn step(&self) -> i64 {
    self
      .interval
      .to_step()
      .expect("Could not convert interval to step")
  }

  fn serialize(
    &self,
    columns: Option<&str>,
  ) -> (String, Vec<Box<(dyn ToSql + Sync)>>) {
    use QueryOpt::*;

    let columns = match columns {
      Some(c) => c,
      _ => Candle::DB_COLUMNS,
    };

    let mut query = format!(
      r#"SELECT {} FROM candles WHERE symbol = {} AND interval = {} AND dead = false"#,
      columns, self.symbol, self.interval
    );
    let params = vec![];
    let mut limit = CONFIG.query_limit;
    let mut order = ASC;
    let mut i = 2;

    for (_, o) in &self.options {
      match o {
        Start(start) => query.push_str(&format!(" AND open_time >= {}", start)),
        End(end) => query.push_str(&format!(" AND close_time >= {}", end)),
        Limit(l) => limit = *l,
        Order(o) => order = o.clone(),
        _ => {}
      };
    }

    query.push_str(match order {
      ASC => "ORDER BY open_time ASC",
      DESC => "ORDER BY open_time DESC",
    });
    query.push_str(format!(" LIMIT {}", limit).as_str());

    (query, params)
  }

  pub fn query_candles(&mut self) -> Result<Vec<Candle>> {
    let (query, params) = self.serialize(None);
    let rows = self.con.query(query.as_str(), &params.to_params())?;
    Ok(rows.iter().enumerate().map(Candle::from).collect())
  }

  pub fn count_candles(&mut self) -> Result<usize> {
    let (query, params) = self.serialize(Some("Count(*)"));
    let rows = self.con.query(query.as_str(), &params.to_params())?;
    Ok(rows[0].get::<usize, i64>(0) as usize)
  }

  pub fn missing_candles(&mut self) -> Result<Vec<Range<i64>>> {
    let step = self.interval.to_step()?;
    let start = match self.get(&Start(0)) {
      Some(Start(s)) => *s,
      _ => bail!("Need a beginning of the range"),
    };
    let end = match self.get(&End(0)) {
      Some(End(e)) => *e,
      _ => bail!("Need and end of the range"),
    };

    let rows = self
          .con.query(
              "
  SELECT c.open_time AS missing_open_times
  FROM generate_series($1::bigint, $2::bigint, $3::bigint) c(open_time)
  WHERE NOT EXISTS (SELECT 1 FROM candles where open_time = c.open_time AND symbol = $4 AND interval = $5);",
              &[&start, &end, &step, &self.symbol, &self.interval],
          )?;

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
  pub fn copy_in_candles(&mut self, out: String) -> Result<()> {
    use std::path::Path;

    fs::create_dir_all("/tmp/pg_copy")?;
    let header = "id, symbol, interval, open_time, open, high, low, close, volume, close_time, bottom_domain, top_domain, fuzzy_domain, dead, indicators";
    let mut _out = String::from(format!("{}\n", header));
    _out.push_str(out.as_str());

    let p = format!("/tmp/pg_copy/{}-{}.csv", self.symbol, self.interval);
    let path = Path::new(&p);
    fs::write(path, out)?;
    self.con.batch_execute("delete from import_candles;")?;
    Command::new("psql")
      .arg("-d")
      .arg(db())
      .arg("-c")
      .arg(&*format!(
        r#"\copy import_candles({header}) FROM '{csv_path}' CSV DELIMITER E'\t' QUOTE '"' ESCAPE '\';"#,
        header = header, csv_path = path.to_str().unwrap()
      ))
      .output()
      .expect("Failed to copy in candles");

    self.con.batch_execute(
      format!(
        "
  DELETE FROM candles WHERE
  open_time IN (SELECT open_time FROM import_candles)
  AND symbol = '{symbol}' AND interval = '{interval}';
  INSERT INTO candles SELECT * FROM import_candles;
  ",
        symbol = self.symbol,
        interval = self.interval
      )
      .as_str(),
    )?;

    Ok(())
  }

  pub fn insert_candle(&mut self, candle: &Candle) -> Result<()> {
    self
      .con
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
          &self.symbol,
          &self.interval,
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
    log!("Database '{}' not found", db());
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
  log!("Done");
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
  log!("Done");
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
  log!("Calculating domain for... {}", interval);
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

trait ToParams<'a> {
  fn to_params(&'a self) -> Vec<&'a (dyn ToSql + Sync)>;
}
impl<'a> ToParams<'a> for Vec<Box<(dyn ToSql + Sync)>> {
  fn to_params(&'a self) -> Vec<&'a (dyn ToSql + Sync)> {
    self.iter().map(|b| &**b).collect()
  }
}
