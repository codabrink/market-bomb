#![macro_use]

mod range;
mod time;

// pub use crate::api;
pub use crate::{
  api::*,
  config::Config,
  core::*,
  database::{self, Order::*, QueryOpt::*, *},
  *,
};
pub use anyhow::{bail, Result};
pub use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
pub use hashbrown::{HashMap, HashSet};
pub use range::*;
pub use serde::{Deserialize, Serialize};
pub use std::{
  fs::{self, File},
  io::Write,
  path::{Path, PathBuf},
  sync::atomic::{AtomicUsize, Ordering::Relaxed},
};
pub use time::*;

pub use postgres::{error::DbError, types::ToSql, Client, NoTls};
pub use r2d2::{Pool, PooledConnection};
pub use r2d2_postgres::PostgresConnectionManager;

lazy_static! {
  pub static ref CONFIG: Config = Config::load();
  pub static ref API: Api = Binance::new();
}

macro_rules! log {
  ($($arg:tt)*) => {
    #[cfg(test)]
    println!($($arg)*);
    #[cfg(not(test))]
    crate::terminal::log(format!($($arg)*));
  };
}
pub fn pb(label: impl AsRef<str>, pct: f64) {
  let _ = terminal::PB.0.send((label.as_ref().to_string(), pct));
}

pub trait Candles {
  fn step(&self) -> i64;
  fn ensure_congruent(&self) -> bool;
}

impl Candles for Vec<Candle> {
  fn step(&self) -> i64 {
    if self.is_empty() {
      return 0;
    }
    &self[0].close_time - &self[0].open_time + 1
  }
  fn ensure_congruent(&self) -> bool {
    if self.is_empty() {
      return true;
    }

    let step = self.step();
    let mut incongruities = 0;

    for i in 0..(self.len() - 1) {
      let expected = self[i].open_time + step;
      let result = self[i + 1].open_time;

      if expected != result {
        incongruities += 1;
        log!(
          "Incongruity at index {} of {}. Expected {}, got {}.",
          i,
          self.len(),
          expected,
          result
        );

        if let Some(_) = self.iter().find(|c| c.open_time == expected) {
          panic!("Candles are out of order");
        }
      }
    }
    assert_eq!(incongruities, 0);

    true
  }
}
