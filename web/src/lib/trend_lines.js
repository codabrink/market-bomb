const colors = [
  'red',
  'black',
  'green',
  'orange',
  'purple',
  'cyan',
  'blue',
  'gray',
]

function rnd(array) {
  return array[Math.floor(Math.random() * array.length)]
}

export default function TrendLines({ chart: { chartBody, y } }) {
  function update({
    state: {
      data: { trend_lines },
      config: { showTrendLines },
    },
    chart: { x, y, t },
  }) {
    chartBody
      .selectAll(`.line`)
      .data(showTrendLines ? trend_lines : [])
      .join(
        (enter) =>
          enter
            .append('line')
            .attr('class', 'line')
            .attr('x1', (d) => x(d.p1[0]))
            .attr('x2', (d) => x(d.p2[0]))
            .attr('y1', (d) => y(d.p1[1]))
            .attr('y2', (d) => y(d.p2[1]))
            .attr('stroke', () => rnd(colors))
            .attr('stroke-width', 1),
        (update) => {
          update.call((update) =>
            update
              .transition(t)
              .attr('x1', (d) => x(d.p1[0]))
              .attr('x2', (d) => x(d.p2[0]))
              .attr('y1', (d) => y(d.p1[1]))
              .attr('y2', (d) => y(d.p2[1]))
          )
        },
        (exit) => exit.remove()
      )
  }

  function zoomed({ xz }) {
    chartBody
      .selectAll('.line')
      .attr('x1', (f) => xz(f.p1[0]))
      .attr('x2', (f) => xz(f.p2[0]))
  }

  function zoomEnd() {
    chartBody
      .selectAll('.line')
      .transition()
      .duration(200)
      .attr('y1', (f) => y(f.p1[1]))
      .attr('y2', (f) => y(f.p2[1]))
  }

  return { zoomed, zoomEnd, update }
}
