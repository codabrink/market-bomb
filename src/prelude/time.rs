use anyhow::{anyhow, Result};
use chrono;
use chrono::Datelike;
use chrono::{prelude::*, Utc};
use regex::Regex;
use std::time::{Duration, UNIX_EPOCH};

const DATETIME_FORMAT: &str = "%m-%d-%y %H:%M:%S";
lazy_static! {
  pub static ref RE_INTERVAL: Regex =
    Regex::new(r"(?P<n>\d+)(?P<unit>[a-zA-Z])").unwrap();
}

pub trait AgoToMs {
  fn ago_ms(&self) -> Result<i64>;
}
impl AgoToMs for &str {
  fn ago_ms(&self) -> Result<i64> {
    let input = self.as_ref();
    let now = chrono::Utc::now();
    let caps = RE_INTERVAL.captures(input).unwrap();
    let mut y = now.year();
    let mut m = now.month();
    let mut d = now.day();
    let mut h = now.hour();

    let n: u32 = caps["n"].parse()?;

    match &caps["unit"] {
      "y" => y -= n as i32,
      "m" => m -= n,
      "d" => d -= n,
      "h" => h -= n,
      _ => (),
    };

    Ok(
      Utc
        .ymd(y, m, d)
        .and_hms(h, 0, 0)
        .to_ms()
        .round("15m".to_step()?),
    )
  }
}

pub fn now() -> i64 { Utc::now().timestamp_millis() as i64 }
pub trait DateTimeToMs {
  fn to_ms(&self) -> i64;
}
impl DateTimeToMs for DateTime<Utc> {
  fn to_ms(&self) -> i64 { self.timestamp_millis() as i64 }
}

pub trait MsExtra {
  fn round(&self, step: i64) -> i64;
  fn to_human(&self) -> String;
}
impl MsExtra for i64 {
  fn round(&self, step: i64) -> i64 { self - self % step }
  fn to_human(&self) -> String {
    let d = UNIX_EPOCH + Duration::from_millis(*self as u64);
    let datetime = DateTime::<Utc>::from(d);
    datetime.format(DATETIME_FORMAT).to_string()
  }
}

pub trait StrToMs {
  fn to_step(&self) -> Result<i64>;
}
impl StrToMs for &str {
  fn to_step(&self) -> Result<i64> {
    let caps = RE_INTERVAL.captures(self).unwrap();
    let q: u128 = caps["n"].parse().unwrap_or(1);

    // get self in seconds
    let milliseconds = match &caps["unit"] {
      "m" => 60,
      "h" => 60 * 60,
      "d" => 60 * 60 * 24,
      "w" => 60 * 60 * 24 * 7,
      v => return Err(anyhow!("{} is not a supported step", v)),
    } * q as i64
      * 1000;

    Ok(match self.chars().nth(0).unwrap() {
      '-' => -milliseconds,
      _ => milliseconds,
    })
  }
}

mod prelude_time_tests {

  #[test]
  fn ms_to_human_works() {
    use crate::prelude::*;
    use chrono::Utc;

    let now = Utc::now();
    let ms = now.timestamp_millis() as i64;

    assert_eq!(ms.to_human(), now.format(time::DATETIME_FORMAT).to_string());
  }
}
