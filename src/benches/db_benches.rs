use crate::{data, database};
use bencher::Bencher;

fn bench_missing_candles(b: &mut Bencher) {
  let pool = database::pool();
  let conn = pool.get().unwrap();
  let interval = "15m";

  let start_date = Utc.ymd(2017, 12, 12).and_hms(12, 0, 0);
  let end_date = Utc.ymd(2017, 12, 12).and_hms(23, 0, 0);

  let start_ms = data::to_ms(start_date, interval);
  let end_ms = data::to_ms(end_date, interval);
  let ms = start_ms..end_ms;
  b.iter(|| database::missing_candles(conn, "BTCUSDT", interval, &ms));
}
