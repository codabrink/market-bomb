#![macro_use]

mod range;
mod time;

// pub use crate::api;
pub use crate::{
  api::*,
  config::Config,
  core::*,
  database::{self, con, Order::*, Query, QueryOpt::*},
  *,
};
pub use ahash::{AHashMap, AHashSet};
pub use anyhow::{bail, Result};
pub use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
pub use range::*;
pub use time::*;

pub use postgres::{types::ToSql, Client, NoTls};
pub use r2d2::{Pool, PooledConnection};
pub use r2d2_postgres::PostgresConnectionManager;
pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;
pub type DbCon = PooledConnection<PostgresConnectionManager<NoTls>>;

lazy_static! {
  pub static ref CONFIG: Config = Config::load();
}

macro_rules! log {
  ($($arg:tt)*) => {
    crate::terminal::log(format!($($arg)*));
  };
}

pub fn test_prep() {
  crate::database::test();
}
