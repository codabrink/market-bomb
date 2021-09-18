import * as d3 from 'd3'
import { filter, debounce } from 'lodash'
import moment from 'moment'
import Candles from './candles'
import TrendLines from './trend_lines'
import TrendLineCrosses from './trend_line_crosses'
import StrongPoints from './strong_points'

// const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
const TIME_FORMAT = 'M/D H:MM'

let chart = {}

const margin = { top: 10, right: 30, bottom: 40, left: 50 }
export function getWH() {
  let container = document.getElementById('chart-container')

  const w = container.offsetWidth - margin.left - margin.right
  const h = container.offsetHeight - margin.top - margin.bottom
  return { w, h }
}

export function zoomEnd({
  state: {
    indicators,
    data: { candles },
  },
  t,
}) {
  let domain = t.rescaleX(chart.x).domain()
  candles = filter(
    candles,
    (c) => c.open_time >= domain[0] && c.open_time <= domain[1]
  )

  for (const indicator of indicators) indicator.zoomEnd({ candles, chart })
}

export function update({ state }) {
  let {
    indicators,
    data: { candles },
  } = state
  if (!candles.length) return

  for (const indicator of indicators) indicator.update({ state, chart })
}

export function init({ setIndicators }) {
  const { w, h } = getWH()

  chart.svg = d3.select('#chart')
  // svg.selectAll('*').remove()
  chart.svg = chart.svg
    .attr('width', w + margin.left + margin.right)
    .attr('height', h + margin.top + margin.bottom)
    .append('g')
    .attr('transform', `translate(${margin.left},${margin.top})`)

  chart.svg
    .append('rect')
    .attr('id', 'rect')
    .attr('width', w)
    .attr('height', h)
    .style('fill', 'none')
    .style('pointer-events', 'all')
    .attr('clip-path', 'url(#clip)')

  chart.gX = chart.svg
    .append('g')
    .attr('class', 'axis x-axis') //Assign "axis" class
    .attr('transform', `translate(0,${h})`)
  chart.gY = chart.svg.append('g').attr('class', 'axis y-axis')

  // gX.selectAll('.tick text').call(wrap, xBand.bandwidth())

  chart.chartBody = chart.svg
    .append('g')
    .attr('class', 'chartBody')
    .attr('clip-path', 'url(#clip)')

  chart.x = d3.scaleLinear().range([0, w]).domain([0, 1])
  chart.xAxis = d3
    .axisBottom()
    .scale(chart.x)
    .tickFormat((t) => moment.unix(t).format(TIME_FORMAT))
  chart.svg
    .selectAll('g.axis.x-axis')
    .attr('transform', `translate(0,${h})`)
    .call(chart.xAxis)

  chart.y = d3.scaleLinear().range([h, 0]).domain([0, 1]) //.nice()
  chart.yAxis = d3.axisLeft().scale(chart.y)
  chart.gY.call(chart.yAxis)
  chart.t = chart.svg.transition().duration(250)

  let chartCandles = Candles({ chart })
  let trendLines = TrendLines({ chart })
  let trendLineCrosses = TrendLineCrosses({ chart })
  let strongPoints = StrongPoints({ chart })
  let indicators = [chartCandles, trendLines, trendLineCrosses, strongPoints]
  setIndicators(indicators)

  chart.svg
    .append('defs')
    .append('clipPath')
    .attr('id', 'clip')
    .append('rect')
    .attr('width', w)
    .attr('height', h)

  const throttledZoomEnd = debounce(
    ({ t }) => {
      window.dispatchEvent(new CustomEvent('zoomEnd', { detail: { t } }))
    },
    200,
    { trailing: true }
  )

  var resizeTimer
  chart.zoom = d3
    .zoom()
    .scaleExtent([1, 100])
    .extent([
      [0, 0],
      [w, h],
    ])
    // .translateExtent([
    //   [-w, 0],
    //   [2 * w, h],
    // ])
    .on('zoom', zoomed)
    .on('zoom.end', function () {
      let t = d3.event.transform
      throttledZoomEnd({ t })
    })
    .on('end', end)

  chart.svg.call(chart.zoom)

  function end() {
    let t = d3.event.transform
    let domain = t.rescaleX(chart.x).domain()

    domain = [Math.round(domain[0]), Math.round(domain[1])]
    // console.log(domain)
    let event = new CustomEvent('setDomain', { detail: { domain } })
    window.dispatchEvent(event)
  }

  function zoomed() {
    var t = d3.event.transform
    let xz = t.rescaleX(chart.x)

    // chart.xAxis.scale(xz).tickFormat((t) => moment.unix(t).format(TIME_FORMAT))

    // gX.call(
    // d3.axisBottom(xz).tickFormat((t, e, target) => {
    // let date = moment.unix(t)
    // return date.format(TIME_FORMAT)
    // })
    // )

    for (const indicator of indicators) {
      indicator.zoomed({ t, xz })
    }
  }
}

function distToSegmentSquared(p, v, w) {
  var l2 = dist2(v, w)

  if (l2 == 0) return dist2(p, v)

  var t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2

  if (t < 0) return dist2(p, v)
  if (t > 1) return dist2(p, w)

  return dist2(p, { x: v.x + t * (w.x - v.x), y: v.y + t * (w.y - v.y) })
}
function distToSegment(p, v, w) {
  return Math.sqrt(distToSegmentSquared(p, v, w))
}
function sqr(x) {
  return x * x
}
function dist2(v, w) {
  return sqr(v.x - w.x) + sqr(v.y - w.y)
}
