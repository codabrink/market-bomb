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
  fn ms(&self) -> i64 {
    *self
  }
  fn ago(&self) -> Self {
    *self
  }
}
impl AsMs for DateTime<Utc> {
  fn ms(&self) -> i64 {
    self.timestamp_millis() as i64
  }
  fn ago(&self) -> i64 {
    now() - self.ms()
  }
}

impl AsMs for String {
  fn ms(&self) -> i64 {
    self.as_str().ms()
  }
  fn ago(&self) -> i64 {
    self.as_str().ago()
  }
}

impl AsMs for &str {
  fn ms(&self) -> i64 {
    let caps = RE_INTERVAL.captures(self).unwrap();
    let q: u128 = caps["n"].parse().unwrap_or(1);

    // get self in seconds
    let milliseconds = match &caps["unit"] {
      "m" => 60,
      "h" => 60 * 60,
      "d" => 60 * 60 * 24,
      "w" => 60 * 60 * 24 * 7,
      v => panic!("{} is not a supported step", v),
    } * q as i64
      * 1000;

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
        return Utc
          .ymd(year as i32, month, now.day())
          .and_hms(now.hour(), now.minute(), 0)
          .ms();
      }
      _ => {
        let mut now = now();
        let n: i64 = caps["n"].parse().unwrap();
        match &caps["unit"] {
          "d" => now -= n * "1d".ms(),
          "h" => now -= n * "1h".ms(),
          "m" => now -= n * "1m".ms(),
          _ => {}
        }
        return now;
      }
    };
  }
}

pub fn now() -> i64 {
  Utc::now().timestamp_millis() as i64
}

pub trait MsExtra {
  fn round(&self, step: impl AsMs) -> i64;
  fn to_human(&self) -> String;
  fn to_datetime(&self) -> DateTime<Utc>;
}
impl MsExtra for i64 {
  fn round(&self, step: impl AsMs) -> i64 {
    let step = step.ms();
    self - self % step
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

mod prelude_time_tests {
  use crate::prelude::*;
  use chrono::Datelike;
  // use chrono::TimeZone;
  use chrono::Utc;

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
    let now = Utc::now();
    let one_y_ago = "1y".ago().to_datetime();

    assert_eq!(now.year() - 1, one_y_ago.year());
    assert_eq!(now.day(), one_y_ago.day());
    assert_eq!(now.month(), one_y_ago.month());
  }
}
