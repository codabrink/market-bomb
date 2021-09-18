import React, { useEffect } from 'react'
import { Select, Switch, Config } from './UI'
import chartHook from './chartHook'

function App() {
  let hook = chartHook()

  useEffect(() => {
    document.addEventListener('keydown', (e) => {
      switch (e.key) {
        case 'ArrowLeft':
          hook.move(-20)
          break
        case 'ArrowRight':
          hook.move(10)
          break
      }
    })
  }, [])

  return (
    <>
      <div id="control-bar" className="p-3 flex">
        <Select
          label="Symbol"
          options={['BTCUSDT', 'ADABTC', 'AXSBTC']}
          value={hook.symbol}
          onChange={hook.setSymbol}
        />
        <Select
          label="Interval"
          options={['1m', '5m', '15m', '30m', '1h', '4h', '1d']}
          value={hook.interval}
          onChange={hook.setInterval}
        />
        {/* <TextInput
          label="Point Percent"
          value={pointPercent}
          onChange={(e) => {
            let pointPercent = e.target.value
            setPointPercent(pointPercent)
            search.pointPercent = pointPercent
            setSearch(search)
            pollUpdate(1500)
          }}
        /> */}

        <Switch
          label="Line Crosses"
          onClick={() => {
            hook.setConfig({
              showCrosses: !hook.config.showCrosses,
            })
          }}
          on={hook.config.showCrosses}
        />
        <Switch
          label="Trend Lines"
          onClick={() => {
            hook.setConfig({
              showTrendLines: !hook.config.showTrendLines,
            })
          }}
          on={hook.config.showTrendLines}
        />
        <Config {...{ hook }} />
      </div>
      <div id="chart-container" className="flex-grow w-full">
        <svg id="chart" />
      </div>
      <div className="absolute bottom-0 right-0 flex">
        {/* {candles.map((c) => (
          <CandleDetail
            key={c.index}
            candle={c}
            onClose={() => {
              setCandles(candles.filter((cc) => cc !== c))
            }}
          />
        ))} */}
      </div>
    </>
  )
}

export default App
