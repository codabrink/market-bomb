import * as d3 from 'd3'
import { getWH } from './chart'

export default function Candles({ chart: { chartBody } }) {
  let { w } = getWH()

  let xBand = d3.scaleBand().range([0, w]).padding(0.3)

  function update({
    state: {
      start,
      end,
      data: {
        candles,
        meta: { step },
      },
    },
    chart: { x, y, gX, gY, xAxis, yAxis, t },
  }) {
    xBand.domain(d3.range(-1, (end - start) / step))
    x.domain([candles[0].open_time, candles[candles.length - 1].open_time])

    let lows = candles.map((f) => f.low)
    let highs = candles.map((f) => f.high)

    y.domain([d3.min(lows), d3.max(highs)])
    gY.call(yAxis)
    gX.call(xAxis)

    chartBody
      .selectAll('.candle')
      .data(candles, (c) => c.open_time)
      .join(
        (enter) =>
          enter
            .append('rect')
            .attr('class', 'candle')
            .attr('x', (d) => x(d.open_time) - xBand.bandwidth())
            .attr('y', (d) => y(Math.max(d.open, d.close)))
            .attr('width', xBand.bandwidth())
            .attr('height', (d) =>
              d.open === d.close
                ? 1
                : y(Math.min(d.open, d.close)) - y(Math.max(d.open, d.close))
            )
            .attr('fill', (d) => (d.open > d.close ? 'red' : 'green'))
            .on('click', (d) => console.log(d)),
        (update) =>
          update.call((update) =>
            update
              .transition(t)
              .attr('x', (d) => x(d.open_time) - xBand.bandwidth())
              .attr('y', (d) => y(Math.max(d.open, d.close)))
              .attr('width', xBand.bandwidth())
              .attr('height', (d) =>
                d.open === d.close
                  ? 1
                  : y(Math.min(d.open, d.close)) - y(Math.max(d.open, d.close))
              )
              .attr('fill', (d) => (d.open > d.close ? 'red' : 'green'))
          ),
        (exit) => exit.remove()
      )

    // .on('click', (d) => setCandles([d], d3.event.shiftKey))
    chartBody
      .selectAll('.stem')
      .data(candles, (c) => c.open_time)
      .join(
        (enter) =>
          enter
            .append('line')
            .attr('class', 'stem')
            .attr('x1', (d) => x(d.open_time) - xBand.bandwidth() / 2)
            .attr('x2', (d) => x(d.open_time) - xBand.bandwidth() / 2)
            .attr('y1', (d) => y(d.high))
            .attr('y2', (d) => y(d.low))
            .attr('stroke', (d) => (d.open > d.close ? 'red' : 'green')),
        (update) =>
          update.call((update) => {
            update
              .transition(t)
              .attr('x1', (d) => x(d.open_time) - xBand.bandwidth() / 2)
              .attr('x2', (d) => x(d.open_time) - xBand.bandwidth() / 2)
              .attr('y1', (d) => y(d.high))
              .attr('y2', (d) => y(d.low))
              .attr('stroke', (d) => (d.open > d.close ? 'red' : 'green'))
          }),
        (exit) => exit.remove()
      )
  }

  function zoomed({ t, xz }) {
    let width = xBand.bandwidth() * t.k
    let halfWidth = width / 2
    chartBody
      .selectAll('.candle')
      .attr('x', (f) => xz(f.open_time) - halfWidth)
      .attr('width', width)
    chartBody
      .selectAll('.stem')
      .attr(
        'x1',
        (f) => xz(f.open_time) - xBand.bandwidth() / 2 + xBand.bandwidth() * 0.5
      )
      .attr(
        'x2',
        (f) => xz(f.open_time) - xBand.bandwidth() / 2 + xBand.bandwidth() * 0.5
      )
  }

  function zoomEnd({ candles, chart: { svg, y, yAxis, gY } }) {
    let min = d3.min(candles, (f) => f.low)
    let max = d3.max(candles, (f) => f.high)
    let buffer = (max - min) * 0.05
    y.domain([min - buffer, max + buffer])
    gY.call(yAxis)

    svg
      .selectAll('.candle')
      .transition()
      .duration(200)
      .attr('y', (d) => y(Math.max(d.open, d.close)))
      .attr('height', (d) =>
        d.open === d.close
          ? 1
          : y(Math.min(d.open, d.close)) - y(Math.max(d.open, d.close))
      )

    svg
      .selectAll('.stem')
      .transition()
      .duration(200)
      .attr('y1', (d) => y(d.high))
      .attr('y2', (d) => y(d.low))
  }

  return {
    zoomed,
    zoomEnd,
    update,
  }
}
