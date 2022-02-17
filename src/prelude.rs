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
pub use serial_test::serial;
pub use std::{fs, sync::atomic::Ordering::Relaxed};
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
    if cfg!(test) {
      println!($($arg)*);
    } else {
      crate::terminal::log(format!($($arg)*));
    }
  };
}
