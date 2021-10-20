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
pub static CANDLES: AtomicUsize = AtomicUsize::new(0);
pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;
pub type DbCon = PooledConnection<PostgresConnectionManager<NoTls>>;

lazy_static! {
  pub static ref POOL: DbPool = init_pool();
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Order {
  ASC,
  DESC,
}

lazy_static! {
  pub static ref DATABASE: RwLock<String> = RwLock::new("trader".into());
}

pub fn candle_counting_thread() {
  thread::spawn(move || loop {
    if let Ok(r) = con().query("select count(*) from candles;", &[]) {
      let v = r[0].get::<usize, i64>(0);
      CANDLES.store(v as usize, Relaxed);
    }
    thread::sleep(Duration::from_secs(2));
  });
}

pub fn setup_test() {
  *DATABASE.write().unwrap() = String::from("trader_test");
  let _ = con().batch_execute("DELETE FROM candles;");
}

#[derive(Clone)]
pub struct Query<'a> {
  pub symbol: &'a str,
  pub interval: &'a str,
  options: AHashSet<QueryOpt>,
}

#[derive(Clone, Debug, Eq)]
pub enum QueryOpt {
  Start(i64),
  End(i64),
  Limit(usize),
  Order(Order),
  TopDomain(i32),
  BottomDomain(i32),
}

impl PartialEq for QueryOpt {
  fn eq(&self, other: &Self) -> bool {
    discriminant(self) == discriminant(other)
  }
}
impl Hash for QueryOpt {
  fn hash<H: Hasher>(&self, state: &mut H) { discriminant(self).hash(state); }
}

impl<'a> Query<'a> {
  pub fn new(symbol: &'a str, interval: &'a str) -> Self {
    Self {
      symbol,
      interval,
      options: AHashSet::new(),
    }
  }

  pub fn get(&self, opt: &QueryOpt) -> Option<&QueryOpt> {
    self.options.get(opt)
  }

  pub fn set(&mut self, opt: QueryOpt) {
    // round time values to interval
    let opt = match opt {
      Start(start) => Start(start.round(self.interval)),
      End(end) => End(end.round(self.interval)),
      v => v,
    };

    self.options.replace(opt);
  }

  pub fn set_all(&mut self, opt: Vec<QueryOpt>) {
    for opt in opt {
      self.set(opt);
    }
  }

  pub fn is_empty(&self) -> bool { self.num_candles() == 0 }

  pub fn num_candles(&self) -> usize {
    match (self.get(&Start(0)), self.get(&End(0))) {
      (Some(Start(start)), Some(End(end))) if start < end => {
        (*start..*end).num_candles(self.step())
      }
      _ => 0,
    }
  }

  pub fn clear(&mut self) { self.options.clear(); }

  pub fn remove(&'a mut self, opt: &QueryOpt) { self.options.remove(&opt); }

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

  pub fn step(&self) -> i64 { self.interval.ms() }

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
      r#"SELECT {} FROM candles WHERE symbol = '{}' AND interval = '{}' AND dead = false"#,
      columns, self.symbol, self.interval
    );
    let params = vec![];
    let mut limit = CONFIG.query_limit;
    let mut order = ASC;

    for o in &self.options {
      match o {
        Start(start) => query.push_str(&format!(" AND open_time >= {}", start)),
        End(end) => query.push_str(&format!(" AND open_time <= {}", end)),
        Limit(l) => limit = *l,
        Order(o) => order = o.clone(),
        _ => {}
      };
    }

    if !columns.to_lowercase().contains("count(*)") {
      query.push_str(match order {
        ASC => " ORDER BY open_time ASC",
        DESC => " ORDER BY open_time DESC",
      });
      query.push_str(format!(" LIMIT {}", limit).as_str());
    }

    // log!("{}", &query);

    (query, params)
  }

  pub fn query_candles(&mut self) -> Result<Vec<Candle>> {
    let (query, params) = self.serialize(None);
    let rows = con().query(query.as_str(), &params.to_params())?;
    Ok(rows.iter().enumerate().map(Candle::from).collect())
  }

  pub fn count_candles(&mut self) -> Result<usize> {
    let (query, params) = self.serialize(Some("COUNT(*)"));
    let rows = con().query(query.as_str(), &params.to_params())?;
    Ok(rows[0].get::<usize, i64>(0) as usize)
  }

  pub fn missing_candles(&mut self) -> Result<Vec<Range<i64>>> {
    let step = self.interval.ms();
    let start = match self.get(&Start(0)) {
      Some(Start(s)) => *s,
      _ => bail!("Need a beginning of the range"),
    };
    let end = match self.get(&End(0)) {
      Some(End(e)) => *e,
      _ => bail!("Need and end of the range"),
    };

    let rows = con().query(
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
    let header = "id, symbol, interval, open_time, open, high, low, close, volume, close_time, bottom_domain, top_domain, fuzzy_domain, dead";
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
  dead,
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
        &candle.dead,
        &"binance",
      ],
    );

    if let Err(e) = result {
      match e.code() {
        Some(&SqlState::UNIQUE_VIOLATION) => {
          UNIQUE_VIOLATIONS.fetch_add(1, Relaxed);
        }
        _ => Err(e)?,
      }
    }

    Ok(())
  }
}

pub fn db() -> String { DATABASE.read().unwrap().clone() }

fn init_pool() -> DbPool {
  if !database_exists() {
    log!("Database '{}' not found", db());
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
  log!("Creating database...");
  Client::connect("host=127.0.0.1 user=postgres", NoTls)?
    .batch_execute(format!("CREATE DATABASE {};", db()).as_str())?;
  log!("Done");
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
}
