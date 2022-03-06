#[macro_use]
extern crate lazy_static;
extern crate env_logger;
extern crate r2d2;

mod prelude;

mod api;
mod config;
mod core;
pub mod database;
mod normalized;
mod strategy;
mod terminal;
mod web_server;

fn main() {
  std::thread::spawn(|| {
    strategy::build_cache("BTCUSDT");
  });

  database::candle_counting_thread();
  terminal::Terminal::new();
}
