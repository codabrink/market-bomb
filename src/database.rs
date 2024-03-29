use crate::prelude::*;
use anyhow::Result;
use postgres::error::SqlState;

use std::{
  hash::{Hash, Hasher},
  mem::discriminant,
  ops::Range,
  process::Command,
  sync::{atomic::AtomicUsize, RwLock},
};

mod migrations;

pub static UNIQUE_VIOLATIONS: AtomicUsize = AtomicUsize::new(0);
pub static DERIVED_CANDLES: AtomicUsize = AtomicUsize::new(0);
pub static CANDLES: AtomicUsize = AtomicUsize::new(0);

pub struct DbPool(Pool<PostgresConnectionManager<NoTls>>);
pub type DbCon = PooledConnection<PostgresConnectionManager<NoTls>>;

lazy_static! {
  pub static ref POOL: RwLock<HashMap<usize, DbPool>> =
    RwLock::new(HashMap::new());
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Order {
  ASC,
  DESC,
}
enum RecordType {
  Candles,
  MovingAverage,
}

fn db() -> String {
  #[cfg(test)]
  return format!("trader_test_{}", thread_id());
  #[cfg(not(test))]
  return "trader".into();
}

pub fn candle_counting_thread() {
  thread::spawn(move || loop {
    if let Ok(r) = con().query("select count(*) from candles;", &[]) {
      let v = r[0].get::<usize, i64>(0);
      CANDLES.store(v as usize, Relaxed);
    }
    if let Ok(r) =
      con().query("select count(*) from candles where derived = true;", &[])
    {
      let v = r[0].get::<usize, i64>(0);
      DERIVED_CANDLES.store(v as usize, Relaxed);
    }
    thread::sleep(Duration::from_secs(2));
  });
}

#[derive(Clone)]
pub struct Query {
  symbol: String,
  interval: String,
  step: i64,
  options: HashMap<&'static str, QueryOpt>,
}

#[derive(Clone, Debug, Eq)]
pub enum QueryOpt {
  Start(i64),
  End(i64),
  Limit(usize),
  Order(Order),
  TopDomain(i32),
  BottomDomain(i32),
  Exp(bool),
  Len(i32),
}

impl From<&QueryOpt> for &str {
  fn from(qo: &QueryOpt) -> Self {
    match qo {
      Start(_) => "start",
      End(_) => "end",
      Limit(_) => "limit",
      Order(_) => "order",
      TopDomain(_) => "top_domain",
      BottomDomain(_) => "bottom_domain",
      Exp(_) => "exp",
      Len(_) => "len",
    }
  }
}

impl PartialEq for QueryOpt {
  fn eq(&self, other: &Self) -> bool {
    discriminant(self) == discriminant(other)
  }
}
impl Hash for QueryOpt {
  fn hash<H: Hasher>(&self, state: &mut H) {
    discriminant(self).hash(state);
  }
}

impl Query {
  pub fn new(symbol: &str, interval: &str) -> Self {
    Self {
      symbol: symbol.to_owned(),
      interval: interval.to_owned(),
      step: interval.ms(),
      options: HashMap::new(),
    }
  }
  pub fn default() -> Self {
    Self::new("BTCUSDT", "15m")
  }

  pub fn get(&self, opt: &QueryOpt) -> Option<&QueryOpt> {
    let s: &str = opt.into();
    self.options.get(s)
  }
  pub fn symbol(&self) -> &str {
    &self.symbol
  }
  pub fn interval(&self) -> &str {
    &self.interval
  }

  pub fn set(&mut self, opt: QueryOpt) {
    // round time values to interval
    let opt = match opt {
      Start(start) => Start(start.round(self.step)),
      End(end) => End(end.round(self.step)),
      v => v,
    };

    let k: &str = (&opt).into();
    self.options.insert(k, opt);
  }

  pub fn set_all(&mut self, opt: Vec<QueryOpt>) {
    for opt in opt {
      self.set(opt);
    }
  }

  pub fn candle_index(&self, candle: &Candle) -> usize {
    match self.start() {
      Some(start) => ((candle.open_time - start) / self.step) as usize,
      _ => 0,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.num_candles() == 0
  }

  pub fn num_candles(&self) -> usize {
    match (self.get(&Start(0)), self.get(&End(0))) {
      (Some(Start(start)), Some(End(end))) if start < end => {
        (*start..*end).num_candles(self.step())
      }
      _ => 0,
    }
  }

  pub fn clear(&mut self) {
    self.options.clear();
  }

  pub fn remove(&mut self, opt: &QueryOpt) {
    let k: &str = opt.into();
    self.options.remove(k);
  }

  pub fn set_interval(&mut self, interval: &str) {
    self.interval = interval.to_owned();
    self.step = self.interval.ms();
  }

  pub fn set_range(&mut self, range: Range<i64>) {
    self.set_all(vec![Start(range.start), End(range.end)]);
  }
  pub fn range(&self) -> Option<Range<i64>> {
    match (self.get(&Start(0)), self.get(&End(0))) {
      (Some(Start(start)), Some(End(end))) => Some(*start..*end),
      _ => None,
    }
  }

  pub fn start(&self) -> Option<i64> {
    if let Some(Start(v)) = self.get(&Start(0)) {
      return Some(*v);
    }
    None
  }

  pub fn end(&self) -> Option<i64> {
    if let Some(End(v)) = self.get(&End(0)) {
      return Some(*v);
    }
    None
  }

  pub fn step(&self) -> i64 {
    self.interval.ms()
  }
  pub fn len(&self) -> Option<i32> {
    if let Some(Len(v)) = self.get(&Len(0)) {
      return Some(*v);
    }
    None
  }
  pub fn exp(&self) -> Option<bool> {
    if let Some(Exp(v)) = self.get(&Exp(false)) {
      return Some(*v);
    }
    None
  }

  pub fn ma_price(
    &self,
    symbol: &str,
    interval: &str,
    ms: i64,
    len: i32,
    exp: bool,
  ) -> Option<f32> {
    let query = format!("SELECT val FROM moving_averages WHERE symbol = '{}' AND INTERVAL = '{}' AND ms <= {} AND exp = {} AND len = {} ORDER BY ms DESC LIMIT 1",symbol, interval, ms, exp, len);
    let rows = con().query(query.as_str(), &[]).unwrap();
    rows.get(0).map(|c| c.get(0))
  }

  pub fn price(&self, open_time: i64) -> Option<f32> {
    let query = format!("SELECT open FROM candles WHERE symbol = '{}' AND INTERVAL = '{}' AND open_time <= {} ORDER BY open_time DESC LIMIT 1", self.symbol, self.interval, open_time);
    let rows = con().query(query.as_str(), &[]).unwrap();
    rows.get(0).map(|c| c.get(0))
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
      r#"SELECT {} FROM candles WHERE symbol = '{}' AND interval = '{}'"#,
      columns, self.symbol, self.interval
    );
    let params = vec![];
    let mut limit = None;
    let mut order = ASC;

    for (_, o) in &self.options {
      match o {
        Start(start) => query.push_str(&format!(" AND open_time >= {}", start)),
        End(end) => query.push_str(&format!(" AND open_time <= {}", end)),
        Limit(l) => limit = Some(*l),
        Order(o) => order = o.clone(),
        _ => {}
      };
    }

    if !columns.to_lowercase().contains("count(*)") {
      query.push_str(match order {
        ASC => " ORDER BY open_time ASC",
        DESC => " ORDER BY open_time DESC",
      });
      if let Some(limit) = limit {
        query.push_str(format!(" LIMIT {}", limit).as_str());
      }
    }

    // log!("{}", &query);

    (query, params)
  }

  pub fn query_candles(&self) -> Result<Vec<Candle>> {
    let (query, params) = self.serialize(None);
    let rows = con().query(query.as_str(), &params.to_params())?;
    Ok(rows.iter().enumerate().map(Candle::from).collect())
  }

  pub fn count_candles(&mut self) -> Result<usize> {
    let (query, params) = self.serialize(Some("COUNT(*)"));
    let rows = con().query(query.as_str(), &params.to_params())?;
    Ok(rows[0].get::<usize, i64>(0) as usize)
  }

  fn missing_ungrouped(&self, record_type: RecordType) -> Result<Vec<i64>> {
    if self.is_empty() {
      return Ok(vec![]);
    }

    let step = self.step();
    let start = match self.get(&Start(0)) {
      Some(Start(s)) => *s,
      _ => bail!("Need a beginning of the range"),
    };
    let end = match self.get(&End(0)) {
      Some(End(e)) => *e - 1,
      _ => bail!("Need and end of the range"),
    };

    let (table, extra) = match record_type {
      RecordType::Candles => ("candles", "".to_owned()),
      RecordType::MovingAverage => (
        "moving_averages",
        format!(
          " AND len={len} AND exp={exp}",
          len = self.len().expect("Needs a len"),
          exp = self.exp().expect("Needs an exp")
        ),
      ),
    };

    let q = format!("
      SELECT c.open_time
      FROM generate_series($1::bigint, $2::bigint, $3::bigint) c(open_time)
      WHERE NOT EXISTS (SELECT 1 FROM {table} where open_time=c.open_time AND symbol='{symbol}' AND interval='{interval}'{extra})",
      table = table, symbol = self.symbol, interval = self.interval, extra = extra);

    let rows = con().query(&q, &[&start, &end, &step])?;

    Ok(rows.iter().map(|i| i.get(0)).collect())
  }

  pub fn missing_ma_ungrouped(&self) -> Result<Vec<i64>> {
    self.missing_ungrouped(RecordType::MovingAverage)
  }

  pub fn missing_candles_ungrouped(&self) -> Result<Vec<i64>> {
    self.missing_ungrouped(RecordType::Candles)
  }

  pub fn is_missing_candles(&self) -> bool {
    !self.missing_candles_ungrouped().unwrap().is_empty()
  }

  pub fn missing_candles(&self) -> Result<Vec<Range<i64>>> {
    let missing = self.missing_candles_ungrouped()?;
    let step = self.interval.ms();

    let mut range_start = 0;
    let mut result = Vec::<Range<i64>>::new();

    // group the missing candles
    for i in 1..missing.len() {
      if missing[i - 1] + step != missing[i] {
        result.push(missing[range_start]..(missing[i - 1] + step));
        range_start = i;
      }
    }

    // push the leftovers
    if missing.len() > range_start {
      result.push(missing[range_start]..(missing[missing.len() - 1] + step));
    }
    Ok(result)
  }
  pub fn copy_in_candles(&mut self, out: String) -> Result<()> {
    fs::create_dir_all("/tmp/pg_copy")?;
    let header = "id, symbol, interval, open_time, open, high, low, close, volume, close_time, bottom_domain, top_domain, fuzzy_domain, derived";
    let mut _out = String::from(format!("{}\n", header));
    _out.push_str(out.as_str());

    let p = format!("/tmp/pg_copy/{}-{}.csv", self.symbol, self.interval);
    let path = Path::new(&p);
    fs::write(path, out)?;
    con().batch_execute("delete from import_candles;")?;
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

    con().batch_execute(
      format!(
        "
DELETE FROM candles WHERE
open_time IN (SELECT open_time FROM import_candles)
AND symbol = '{symbol}' AND interval = '{interval}';
INSERT INTO candles SELECT * FROM import_candles;",
        symbol = self.symbol,
        interval = self.interval
      )
      .as_str(),
    )?;

    Ok(())
  }

  pub fn insert_candle(&mut self, candle: &Candle) -> Result<()> {
    let result = con().execute(
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
  derived,
  source
) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
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
        &candle.derived,
        &"binance",
      ],
    );

    if let Err(e) = result {
      match e.code() {
        Some(&SqlState::UNIQUE_VIOLATION) => {
          // maybe we'll want to do a replace in the future
          // but for now, let's just (mostly) ignore it.
          log!("Unique violation.");
          UNIQUE_VIOLATIONS.fetch_add(1, Relaxed);
        }
        _ => Err(e)?,
      }
    }

    Ok(())
  }

  fn known_siblings(
    &mut self,
    open_time: i64,
  ) -> Result<(Option<Candle>, Option<Candle>)> {
    let r = con().query(
      format!(
        "
(SELECT {cols} FROM candles WHERE open_time < {ot} ORDER BY open_time DESC LIMIT 1)
UNION ALL
(SELECT {cols} FROM candles WHERE open_time > {ot} ORDER BY open_time ASC LIMIT 1)
",
        cols = Candle::DB_COLUMNS,
        ot = open_time
      )
      .as_str(),
      &[],
    )?;

    Ok((r.get(0).map(Candle::from), r.get(1).map(Candle::from)))
  }

  pub fn linear_regression(&mut self) -> Result<()> {
    con().batch_execute(format!("DELETE FROM candles WHERE symbol = '{}' AND INTERVAL = '{}' AND derived = true", self.symbol, self.interval).as_str())?;

    let missing = self.missing_candles_ungrouped()?;
    log!(
      "Calculating linear regression for {} candles...",
      missing.len()
    );
    for open_time in missing {
      if let (Some(left), Some(right)) = self.known_siblings(open_time)? {
        let dl = (open_time - left.open_time) as f32;
        let dr = (right.open_time - open_time) as f32;
        let dt = dl + dr;

        // get the fractions
        let dl = 1. - (dl / dt);
        let dr = 1. - (dr / dt);

        let candle = Candle {
          open: left.open * dl + right.open * dr,
          high: left.high * dl + right.high * dr,
          low: left.low * dl + right.low * dr,
          close: left.close * dl + right.close * dr,
          volume: left.volume * dl + right.volume * dr,
          open_time,
          close_time: open_time + self.step() - 1,
          derived: true,
          ..Default::default()
        };

        self.insert_candle(&candle)?;
      }
    }

    Ok(())
  }
}

fn init_pool() -> DbPool {
  // test cleanup
  #[cfg(test)]
  let _ = drop_db();

  if !database_exists() {
    if let Err(err) = create_db() {
      log!("Create db: {:?}", err);
    }
    if let Err(err) = migrate_db() {
      log!("Migrate db: {:?}", err);
    }
  }
  let manager = r2d2_postgres::PostgresConnectionManager::new(
    format!("host=127.0.0.1 user=postgres dbname={}", db())
      .parse()
      .unwrap(),
    NoTls,
  );
  DbPool(Pool::new(manager).unwrap())
}

pub fn thread_id() -> usize {
  thread_id::get()
}

pub fn con() -> DbCon {
  if let Some(pool) = POOL.read().unwrap().get(&thread_id()) {
    return pool.0.get().unwrap();
  }

  let mut p = POOL.write().unwrap();
  p.insert(thread_id(), init_pool());
  drop(p);

  return con();
}

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
pub fn reset() {
  con().batch_execute("DELETE FROM candles;").unwrap();
}
pub fn create_db() -> Result<()> {
  #[cfg(not(test))]
  log!("Creating database...");
  Client::connect("host=127.0.0.1 user=postgres", NoTls)?
    .batch_execute(format!("CREATE DATABASE {};", db()).as_str())?;
  #[cfg(not(test))]
  log!("Done");
  Ok(())
}
pub fn drop_db() -> Result<()> {
  Client::connect("host=127.0.0.1 user=postgres", NoTls)?
    .batch_execute(format!("DROP DATABASE {};", db()).as_str())?;
  Ok(())
}

fn migrate_db() -> Result<()> {
  log!("Migrating database...");
  log!("Creating candles...");
  migrations::create_candles_table()?;
  log!("Creating moving averages...");
  migrations::create_moving_averages_table()?;
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

pub fn calculate_domain(con: &mut DbCon, interval: &str) -> Result<()> {
  log!("Calculating domain for... {}", interval);
  let step = interval.ms();
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
  let day_step = "1d".ms();
  let day_ms = open_time.round(day_step);

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

#[cfg(test)]
mod tests {
  use crate::prelude::*;

  #[test]
  fn query_works() -> Result<()> {
    let mut query = Query::new("BTCUSDT", "15m");
    query.set_all(vec![Start("1h".ago()), End("0m".ago())]);

    assert!(!query.is_empty());

    Ok(())
  }

  #[test]
  fn linear_regression() -> Result<()> {
    let mut query = Query::default();
    let step = query.step();

    let c1 = Candle {
      open: 300.,
      high: 400.,
      low: 100.,
      close: 200.,
      volume: 100.,
      open_time: "3h".ago().round(step),
      close_time: "3h".ago().round(step) + step,
      ..Default::default()
    };

    let c2 = Candle {
      open: 75.,
      high: 100.,
      low: 25.,
      close: 50.,
      volume: 50.,
      open_time: "1h".ago().round(step),
      close_time: "1h".ago().round(step) + step,
      ..Default::default()
    };

    query.insert_candle(&c1)?;
    query.insert_candle(&c2)?;

    // red herrings
    query.insert_candle(&Candle {
      open_time: "1d".ago().round(step),
      ..Default::default()
    })?;
    query.insert_candle(&Candle {
      open_time: "15m".ago().round(step),
      ..Default::default()
    })?;

    // ensure that known_siblings works first
    match query.known_siblings("2h".ago())? {
      (Some(left), Some(right)) => {
        assert_eq!(left.open_time, c1.open_time);
        assert_eq!(right.open_time, c2.open_time);
      }
      _ => bail!("known_siblings is not returning two candles as expected."),
    }

    query.set_all(vec![Start(c1.open_time), End(c2.open_time)]);
    query.linear_regression()?;
    let count = query.count_candles()?;

    // 3h      2h      1h
    // | | | | | | | | |
    assert_eq!(count, 9);

    query.set_all(vec![Start(c1.open_time + step), End(c1.open_time + step)]);
    let candles = query.query_candles()?;
    assert_eq!(candles.len(), 1);

    let one_eigth = 1. / 8.;
    let seven_eighths = 7. / 8.;

    assert_eq!(
      candles[0].open,
      c1.open * seven_eighths + c2.open * one_eigth
    );
    assert_eq!(
      candles[0].high,
      c1.high * seven_eighths + c2.high * one_eigth
    );
    assert_eq!(
      candles[0].close,
      c1.close * seven_eighths + c2.close * one_eigth
    );
    assert_eq!(candles[0].low, c1.low * seven_eighths + c2.low * one_eigth);

    Ok(())
  }
}
