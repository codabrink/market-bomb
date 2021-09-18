export default function StrongPoints({ chart: { chartBody, y } }) {
  function update({
    state: {
      data: { strong_points },
    },
    chart: { x, y, t },
  }) {
    chartBody
      .selectAll('.point')
      .data(strong_points)
      .join(
        (enter) =>
          enter
            .append('circle')
            .attr('class', 'point')
            .attr('cx', (p) => x(p.x))
            .attr('cy', (p) => y(p.y))
            .attr('r', 2)
            .attr('fill', 'white'),
        (update) =>
          update.call((update) =>
            update
              .transition(t)
              .attr('cx', (p) => x(p.x))
              .attr('cy', (p) => y(p.y))
              .attr('fill', 'white')
          ),
        (exit) => exit.remove()
      )
  }

  function zoomed({ xz }) {
    chartBody.selectAll('.point').attr('cx', (p) => xz(p.x))
  }
  function zoomEnd() {
    chartBody
      .selectAll('.point')
      .transition()
      .duration(200)
      .attr('cy', (p) => y(p.y))
  }
  return { zoomed, zoomEnd, update }
}
