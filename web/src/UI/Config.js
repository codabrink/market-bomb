import React, { useState } from 'react'
import classNames from 'classnames'
import queryString from 'query-string'

export default function Config({ hook }) {
  let search = queryString.parse(window.location.search)

  let [open, setOpen] = useState(false)
  let [minDomain, setMinDomain] = useState(search.min_domain)

  return (
    <div className="relative">
      <div
        className="p-3 cursor-pointer bg-blue-500 rounded-sm text-white"
        onClick={() => setOpen(!open)}
      >
        Config
      </div>
      <div
        className={classNames(
          { hidden: !open },
          'top-full bg-white text-black p-4 absolute z-10'
        )}
      >
        <h6 className="font-bold text-xs pb-3">Strong Point</h6>
        <input
          placeholder="Domain limit"
          type="number"
          onChange={(e) => setMinDomain(e.target.value)}
          value={minDomain}
          onKeyPress={(e) => {
            if (e.key === 'Enter') {
              console.log('Set min domain')
              hook.setConfig('strong_point.min_domain', minDomain)
            }
          }}
        />
      </div>
      <div
        onClick={() => setOpen(false)}
        className={classNames(
          { hidden: !open },
          'fixed h-full w-full top-0 left-0 z-0'
        )}
      />
    </div>
  )
}
