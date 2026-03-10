import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getCandles } from '@/services/indexer/client'
import type { IndexerCandle } from '@/types'

const INTERVALS = ['1m', '5m', '15m', '1h', '4h', '1d'] as const

interface PriceChartProps {
  pairAddress: string
  defaultInterval?: string
}

export default function PriceChart({ pairAddress, defaultInterval = '1h' }: PriceChartProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<any>(null)
  const seriesRef = useRef<any>(null)
  const [interval, setInterval_] = useState(defaultInterval)

  const candlesQuery = useQuery({
    queryKey: ['candles', pairAddress, interval],
    queryFn: () => getCandles(pairAddress, interval),
    refetchInterval: 30_000,
    enabled: !!pairAddress,
  })

  useEffect(() => {
    let chart: any = null

    async function initChart() {
      if (!containerRef.current) return

      const lc = await import('lightweight-charts')
      chart = lc.createChart(containerRef.current, {
        layout: {
          background: { color: 'transparent' },
          textColor: '#9ca3af',
        },
        grid: {
          vertLines: { color: 'rgba(255,255,255,0.04)' },
          horzLines: { color: 'rgba(255,255,255,0.04)' },
        },
        crosshair: {
          mode: 0,
        },
        rightPriceScale: {
          borderColor: 'rgba(255,255,255,0.1)',
        },
        timeScale: {
          borderColor: 'rgba(255,255,255,0.1)',
          timeVisible: true,
          secondsVisible: false,
        },
        width: containerRef.current.clientWidth,
        height: 400,
      })

      seriesRef.current = chart.addCandlestickSeries({
        upColor: '#22c55e',
        downColor: '#ef4444',
        borderDownColor: '#ef4444',
        borderUpColor: '#22c55e',
        wickDownColor: '#ef4444',
        wickUpColor: '#22c55e',
      })

      chartRef.current = chart

      const resizeObserver = new ResizeObserver(() => {
        if (containerRef.current) {
          chart.applyOptions({ width: containerRef.current.clientWidth })
        }
      })
      resizeObserver.observe(containerRef.current)

      return () => resizeObserver.disconnect()
    }

    initChart()

    return () => {
      if (chart) {
        chart.remove()
        chartRef.current = null
        seriesRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (!seriesRef.current || !candlesQuery.data) return

    const data = candlesQuery.data
      .filter((c: IndexerCandle) => c.open_price && c.close_price)
      .map((c: IndexerCandle) => ({
        time: Math.floor(new Date(c.open_time).getTime() / 1000) as any,
        open: parseFloat(c.open_price),
        high: parseFloat(c.high_price),
        low: parseFloat(c.low_price),
        close: parseFloat(c.close_price),
      }))
      .sort((a: any, b: any) => a.time - b.time)

    seriesRef.current.setData(data)
    chartRef.current?.timeScale().fitContent()
  }, [candlesQuery.data])

  return (
    <div className="bg-dex-card rounded-2xl border border-dex-border p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-gray-300">Price Chart</h3>
        <div className="flex gap-1">
          {INTERVALS.map((iv) => (
            <button
              key={iv}
              onClick={() => setInterval_(iv)}
              className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                interval === iv
                  ? 'bg-dex-accent text-dex-bg'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {iv}
            </button>
          ))}
        </div>
      </div>

      {candlesQuery.isLoading && (
        <div className="flex items-center justify-center h-[400px] text-gray-500">
          Loading chart...
        </div>
      )}

      {candlesQuery.isError && (
        <div className="flex items-center justify-center h-[400px] text-red-400 text-sm">
          Failed to load chart data
        </div>
      )}

      <div
        ref={containerRef}
        className={candlesQuery.isLoading || candlesQuery.isError ? 'hidden' : ''}
      />
    </div>
  )
}
