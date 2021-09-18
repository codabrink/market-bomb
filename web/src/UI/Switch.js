import React from 'react'
import classNames from 'classnames'

export default function Switch({ on, onClick, label }) {
  return (
    <div className="flex items-center px-4">
      <button
        type="button"
        aria-pressed="false"
        aria-labelledby="toggleLabel"
        className={`${classNames({
          'bg-gray-200': !on,
          'bg-indigo-600': on,
        })} relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500`}
        onClick={onClick}
      >
        <span className="sr-only">Use setting</span>
        <span
          aria-hidden="true"
          className={`${classNames({
            'translate-x-5': on,
            'translate-x-0': !on,
          })} inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200`}
        ></span>
      </button>
      <span className="ml-3" id="toggleLabel">
        <span className="text-sm font-medium text-white">{label}</span>
        {/* <span className="text-sm text-gray-500">(Save 10%)</span> */}
      </span>
    </div>
  )
}
