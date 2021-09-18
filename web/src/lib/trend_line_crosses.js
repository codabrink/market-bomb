import { join } from 'lodash'

const arrowOffset = 0.2

export default function TrendLineCrosses({ chart: { svg, y } }) {
  function update({
    state: {
      data: { trend_lines },
      config: { showCrosses },
    },
    chart: { x, y, t },
  }) {
    let crosses = showCrosses
      ? trend_lines.reduce((acc, v) => [...acc, ...v.crosses], [])
      : []

    svg
      .selectAll('.cross')
      .data(crosses)
      .join(
        (enter) => {
          enter
            .append('line')
            .attr('class', 'cross')
            .attr('x1', (c) => x(c.p1[0]))
            .attr('x2', (c) => x(c.p2[0]))
            .attr('y1', (c) => y(c.p1[1]))
            .attr('y2', (c) => y(c.p2[1]))
            .attr('stroke', 'white')
            .attr('stroke-width', 4)
            .on('click', function (c) {
              console.log(c)
            })
        },
        (update) =>
          update
            .call((update) =>
              update
                .transition(t)
                .attr('x1', (c) => x(c.p1[0]))
                .attr('x2', (c) => x(c.p2[0]))
                .attr('y1', (c) => y(c.p1[1]))
                .attr('y2', (c) => y(c.p2[1]))
            )
            .on('click', function (c) {
              console.log(c)
            }),
        (exit) => exit.remove()
      )

    svg
      .selectAll('.arrow')
      .data(crosses)
      .join(
        (enter) => {
          enter
            .append('text')
            .attr('class', 'arrow')
            .attr('x', (c) => x(c.p2[0] + arrowOffset))
            .attr('y', (c) => y(c.p2[1]))
            .attr('dy', '0.35em')
            .text((c) => (c.t === 'REJECT' || c.t === 'DOWN' ? '↓' : '↑'))
            .attr('fill', 'white')
            .attr('font-size', 18)
            .on('click', function (c) {
              console.log(c)
            })
        },
        (update) =>
          update
            .call((update) =>
              update
                .transition(t)
                .attr('x', (c) => x(c.p2[0] + arrowOffset))
                .attr('y', (c) => y(c.p2[1]))
                .text((c) => (c.t === 'REJECT' || c.t === 'DOWN' ? '↓' : '↑'))
            )
            .on('click', function (c) {
              console.log(c)
            }),
        (exit) => {
          exit.remove()
        }
      )
  }

  function zoomed({ xz }) {
    svg
      .selectAll('.cross')
      .attr('x1', (c) => xz(c.p1[0]))
      .attr('x2', (c) => xz(c.p2[0]))
    svg.selectAll('.arrow').attr('x', (c) => xz(c.p2[0] + arrowOffset))
  }

  function zoomEnd() {
    svg
      .selectAll('.cross')
      .transition()
      .duration(200)
      .attr('y1', (c) => y(c.p1[1]))
      .attr('y2', (c) => y(c.p2[1]))
    svg
      .selectAll('.arrow')
      .transition()
      .duration(200)
      .attr('y', (c) => y(c.p2[1]))
  }

  return { zoomed, zoomEnd, update }
}
