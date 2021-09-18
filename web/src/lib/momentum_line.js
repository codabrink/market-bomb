import * as d3 from 'd3'
import { getWH } from './chart'

export default function momentum({ svg, data, x }) {
  let { frames } = data
  const values = frames.map((f) => f.momentum)
  const { w, h } = getWH()

  let y = d3
    .scaleLinear()
    .domain([d3.min(values), d3.max(values)])
    .range([h, 0])
    .nice()
  let gY = svg.append('g').call(d3.axisRight(y))

  let zeroLine = svg
    .append('line')
    .attr('class', 'momentum-zero')
    .style('stroke', 'black')
    .style('stroke-width', 1)
    .attr('x1', 20)
    .attr('x2', w)
    .attr('y1', y(0))
    .attr('y2', y(0))

  let group = svg.append('g')
  group
    .append('path')
    .datum(values)
    .attr('class', 'line')
    .attr('fill', 'none')
    .attr('stroke', 'steelblue')
    .attr('opacity', 0.5)
    .attr('stroke-width', 1.5)
    .attr(
      'd',
      d3
        .line()
        .x((_, i) => x(i))
        .y(y)
    )

  function zoomed({ xz }) {
    group.select('.line').attr(
      'd',
      d3
        .line()
        .x((_, i) => xz(i))
        .y(y)
    )
  }

  function zoomend({ frames, xz }) {
    let _values = frames.map((d) => d.momentum)
    y.domain([d3.min(_values), d3.max(_values)])
    gY.call(d3.axisRight().scale(y))

    group
      .select('.line')
      .transition()
      .duration(200)
      .attr(
        'd',
        d3
          .line()
          .x((_, i) => xz(i))
          .y(y)
      )

    zeroLine.attr('y1', y(0)).attr('y2', y(0))
  }

  return {
    zoomed,
    zoomend,
  }
}
