#[derive(Copy, Clone, Debug)]
struct Point {
  x: f32,
  y: f32,
}

impl Point {
  pub fn new(x: f32, y: f32) -> Self { Point { x, y } }
}

#[derive(Copy, Clone, Debug)]
struct Line(Point, Point);

impl Line {
  pub fn intersect(self, other: Self) -> Option<Point> {
    let a1 = self.1.y - self.0.y;
    let b1 = self.0.x - self.1.x;
    let c1 = a1 * self.0.x + b1 * self.0.y;

    let a2 = other.1.y - other.0.y;
    let b2 = other.0.x - other.1.x;
    let c2 = a2 * other.0.x + b2 * other.0.y;

    let delta = a1 * b2 - a2 * b1;

    if delta == 0.0 {
      return None;
    }

    Some(Point {
      x: (b2 * c1 - b1 * c2) / delta,
      y: (a1 * c2 - a2 * c1) / delta,
    })
  }
}
