import { useEffect, useReducer } from 'react'
import { init, update, zoomEnd } from './lib/chart'
import queryString from 'query-string'
import { sortBy, uniqBy, set } from 'lodash'
import * as d3 from 'd3'

let lastFetch = 0

export default function ChartHook() {
  let search = queryString.parse(window.location.search)

  let initState = {
    indicators: [],
    interval: search.interval || '15m',
    symbol: search.symbol || 'BTCUSDT',
    pointPercent: 0.009,
    start: search.start || 0,
    end: search.end || 0,
    config: {
      showCrosses: false,
      showTrendLines: true,
      strong_point: {
        limit_by: 'FIXED',
        count: 200,
        min_domain: search.min_domain,
      },
    },
    data: {
      candles: [],
      meta: {},
      strong_points: [],
      trend_lines: [],
    },
  }
  let [state, dispatch] = useReducer(reducer, initState)

  useEffect(() => {
    setTimeout(() => {
      init({
        state,
        setIndicators: (indicators) =>
          dispatch({ type: 'setIndicators', indicators }),
      })
    }, 0)
  }, [])

  const zoomed = ({ t, xz }) => {
    for (const indicator of state.indicators) indicator.zoomed({ t, xz })
  }

  useEffect(() => {
    console.log('state: ', state)
    let _setDomain = (e) => {
      dispatch({ type: 'setDomain', domain: e.detail.domain })
    }

    let _zoomed = (e) => zoomed({ ...e.detail })
    let _zoomEnd = (e) => zoomEnd({ state, t: e.detail.t })

    // window.addEventListener('resize', _redraw)
    window.addEventListener('setDomain', _setDomain)
    window.addEventListener('zoomed', _zoomed)
    window.addEventListener('zoomEnd', _zoomEnd)
    return () => {
      // window.removeEventListener('resize', _redraw)
      window.removeEventListener('setDomain', _setDomain)
      window.removeEventListener('zoomed', _zoomed)
      window.removeEventListener('zoomEnd', _zoomEnd)
    }
  }, [state])

  useEffect(() => {
    let now = new Date().getTime()
    if (now - lastFetch < 500) return
    lastFetch = now

    let search = queryString.stringify({
      symbol: state.symbol,
      interval: state.interval,
      min_domain: state.config.strong_point.min_domain,
    })

    window.history.replaceState(
      null,
      '',
      `${window.location.pathname}?${search}`
    )
    let url = `/chart?${search}`

    let data = fetch(url)
      .then((r) => r.json())
      .then((data) => {
        for (const candle of data.candles.candles) {
          candle.Date = d3.timeParse('%Q')(candle.open_time)
        }
        dispatch({ type: 'loadData', data })
      })
  }, [
    state.symbol,
    state.interval,
    state.pointPercent,
    state.config.strong_point.min_domain,
  ])

  useEffect(() => {
    update({ state })
  }, [state.data, state.config])

  return {
    ...state,
    move: (candles) => dispatch({ type: 'move', candles }),
    setSymbol: (symbol) => dispatch({ type: 'setSymbol', symbol }),
    setInterval: (interval) => dispatch({ type: 'setInterval', interval }),
    setConfig: (k, v) => dispatch({ type: 'setConfig', k, v }),
    appendState: (state) => dispatch({ type: 'appendState', state }),
  }
}

function reducer(state, action) {
  switch (action.type) {
    case 'loadData':
      return {
        ...state,
        start: action.data.meta.start,
        end: action.data.meta.end,
        data: {
          ...action.data,
          candles: sortBy(
            uniqBy(
              [...state.data.candles, ...action.data.candles.candles],
              (c) => c.open_time
            ),
            (c) => c.open_time
          ),
        },
      }
    case 'setIndicators':
      return {
        ...state,
        indicators: [...action.indicators],
      }
    case 'move':
      let step = state.data.meta.step * action.candles
      return {
        ...state,
        start: state.start + step,
        end: state.end + step,
      }
    case 'setSymbol':
      return {
        ...state,
        symbol: action.symbol,
      }
    case 'setInterval':
      return {
        ...state,
        interval: action.interval,
        start: null,
        end: null,
      }
    case 'setConfig':
      let config = state.config
      set(config, action.k, action.v)
      return {
        ...state,
        config,
      }
    case 'setDomain':
      return {
        ...state,
        start: action.domain[0],
        end: action.domain[1],
      }
    default:
      return state
  }
}
