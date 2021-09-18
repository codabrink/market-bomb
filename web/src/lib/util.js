export function project(point, theta, length) {
  let { x, y } = point
  let dx = length * Math.cos(theta)
  let dy = length * Math.sin(theta)
  return { x: x + dx, y: y + dy }
}
