use chrono;
use chrono::Datelike;
use chrono::{prelude::*, Utc};
use regex::Regex;
use std::time::{Duration, UNIX_EPOCH};

const DATETIME_FORMAT: &str = "%m/%d/%y %H:%M";
lazy_static! {
  pub static ref RE_INTERVAL: Regex =
    Regex::new(r"(?P<n>\d+)(?P<unit>[a-zA-Z])").unwrap();
}

pub trait AsMs {
  fn ms(&self) -> i64;
  fn ago(&self) -> i64;
}

impl AsMs for i64 {
  fn ms(&self) -> i64 { *self }
  fn ago(&self) -> Self { *self }
}
impl AsMs for DateTime<Utc> {
  fn ms(&self) -> i64 { self.timestamp_millis() as i64 }
  fn ago(&self) -> i64 { now() - self.ms() }
}

impl AsMs for String {
  fn ms(&self) -> i64 { self.as_str().ms() }
  fn ago(&self) -> i64 { self.as_str().ago() }
}

impl AsMs for &str {
  fn ms(&self) -> i64 {
    let caps = RE_INTERVAL.captures(self).unwrap();
    let q: u128 = caps["n"].parse().unwrap_or(1);

    // get self in seconds
    let milliseconds = match &caps["unit"] {
      "m" => 1,
      "h" => 60,
      "d" => 60 * 24,
      "w" => 60 * 24 * 7,
      "M" => 60 * 24 * 30,
      "y" => 60 * 24 * 365,
      v => panic!("{} is not a supported step", v),
    } * q as i64
      * 60000;

    match self.chars().nth(0).unwrap() {
      '-' => -milliseconds,
      _ => milliseconds,
    }
  }
  fn ago(&self) -> i64 {
    let input = self.as_ref();
    let caps = RE_INTERVAL.captures(input).unwrap();

    match &caps["unit"] {
      "y" | "M" => {
        let now = chrono::Utc::now();
        let n: u32 = caps["n"].parse().unwrap();
        let mut year = now.year() as u32;
        let mut month = now.month();
        match &caps["unit"] {
          "y" => year -= n,
          "M" => {
            year -= n / 12;
            month -= n % 12;
          }
          _ => unreachable!(),
        }
        Utc
          .ymd(year as i32, month, now.day())
          .and_hms(now.hour(), now.minute(), 0)
          .ms()
      }
      _ => now() - input.ms(),
    }
  }
}

pub fn now() -> i64 { Utc::now().timestamp_millis() as i64 }

const WEEK_MS: i64 = 604800000;

pub trait MsExtra {
  fn round(&self, step: impl AsMs) -> i64;
  fn to_human(&self) -> String;
  fn to_datetime(&self) -> DateTime<Utc>;
}
impl MsExtra for i64 {
  fn round(&self, step: impl AsMs) -> i64 {
    let step = step.ms();
    match step {
      // epoch was on a Thursday, so this must be adjusted
      WEEK_MS => self - self % step + "4d".ms(),
      _ => self - self % step,
    }
  }
  fn to_datetime(&self) -> DateTime<Utc> {
    let d = UNIX_EPOCH + Duration::from_millis(*self as u64);
    DateTime::<Utc>::from(d)
  }
  fn to_human(&self) -> String {
    let d = UNIX_EPOCH + Duration::from_millis(*self as u64);
    let datetime = DateTime::<Utc>::from(d);
    datetime.format(DATETIME_FORMAT).to_string()
  }
}

#[cfg(test)]
mod prelude_time_tests {

  #[test]
  fn ms_to_human_works() {
    use crate::prelude::*;
    use chrono::Utc;

    let now = Utc::now();
    let ms = now.timestamp_millis() as i64;

    assert_eq!(ms.to_human(), now.format(time::DATETIME_FORMAT).to_string());
  }

  #[test]
  fn ago_works() {
    use crate::prelude::*;
    use chrono::Datelike;
    use chrono::Utc;

    let now = Utc::now();
    let one_y_ago = "1y".ago().to_datetime();

    assert_eq!(now.year() - 1, one_y_ago.year());
    assert_eq!(now.day(), one_y_ago.day());
    assert_eq!(now.month(), one_y_ago.month());
  }

  #[test]
  fn time_weeks() {
    use crate::prelude::*;

    let start = "7w".ago();
    let end = "1w".ago();

    println!("start: {}", start);
    println!("end:   {}", end);

    assert_eq!((start..end).num_candles("1w"), 6);
  }
}
