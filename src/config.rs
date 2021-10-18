use crate::prelude::*;
use std::fs;
use std::path::Path;

const CONF_FILE: &str = "config.json";

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub exchange_fee: f32,
  pub transaction_slippage: f32,
  pub query_limit: usize,
  // Percent profit expected to vote to take the trade
  pub min_profit: f32,
  // milliseconds expected to sit in a trade
  pub trade_duration_ms: i64,
  pub history_num_candles: i64,
  pub strong_points: StrongPointsConfig,
  pub export: ExportConfig,
}
#[derive(Serialize, Deserialize)]
pub struct StrongPointsConfig {
  pub min_domain: i32,
}
#[derive(Serialize, Deserialize)]
pub struct ExportConfig {
  pub strong_point_length: usize,
  pub detail_view_len: usize,
  pub predict_candles_forward: usize,
}

impl ::std::default::Default for Config {
  fn default() -> Self {
    Self {
      exchange_fee: 0.001,
      transaction_slippage: 0.01,
      query_limit: 2000,
      min_profit: 0.1,
      trade_duration_ms: "1d".ms(),
      history_num_candles: 10000,
      export: ExportConfig {
        detail_view_len: 32,
        strong_point_length: 100,
        predict_candles_forward: 32,
      },
      strong_points: StrongPointsConfig { min_domain: 4 },
    }
  }
}

impl Config {
  pub fn load() -> Self {
    default();

    let config: Config = serde_json::from_str(
      &fs::read_to_string(CONF_FILE).expect("Config file not found."),
    )
    .expect("Could not parse config file.");
    config
  }
  pub fn export_detail_len(&self) -> usize {
    self.export.detail_view_len
  }
  pub fn export_sp_len(&self) -> usize {
    self.export.strong_point_length
  }
  pub fn predict_candles_forward(&self) -> usize {
    self.export.predict_candles_forward
  }
}
fn default() {
  if Path::new(&CONF_FILE).exists() {
    return;
  }
  let config = Config {
    ..Default::default()
  };

  fs::write(CONF_FILE, serde_json::to_string(&config).unwrap())
    .expect("Could not write default config.");
}
