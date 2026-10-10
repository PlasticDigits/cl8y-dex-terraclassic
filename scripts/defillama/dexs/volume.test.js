'use strict'
const { test } = require('node:test')
const assert = require('node:assert/strict')
const { getDailyVolume } = require('./volume.ts')
const ts = 1790208000
const day = '2026-09-24'
const daily = { timestamp: ts, date: day, trade_count: 28, volume_usd: null }
const series = { grain: 'daily', timezone: 'UTC', methodology: 'protocol_catalog',
  series: [{ utc_day: day, trade_count: 28, volume_usd: '558.461005050729' }] }

function get(feed = daily, rollup = series) {
  return async url => url.includes('/defillama/daily?') ? feed : rollup
}

test('legacy partial day recovers the matching priced lower bound', async () => {
  assert.equal(await getDailyVolume(ts, day, get()), 558.461005050729)
})
test('valid feed needs no fallback; idle zero stays zero', async () => {
  for (const [volume_usd, trade_count] of [['10', 28], ['0', 0]]) {
    let calls = 0
    const value = await getDailyVolume(ts, day, async () => {
      calls++
      return { ...daily, volume_usd, trade_count }
    })
    assert.equal(value, Number(volume_usd))
    assert.equal(calls, 1)
  }
})
test('broader protocol count cannot admit an excluded gem swap', async () => {
  await assert.rejects(getDailyVolume(ts, day, get(daily, { ...series,
    series: [{ ...series.series[0], trade_count: 29 }] })))
})
test('missing, duplicate, all-unpriced and wrong-day fallback rows reject', async () => {
  for (const rows of [[], [series.series[0], series.series[0]],
    [{ ...series.series[0], utc_day: '2026-09-23' }],
    [{ ...series.series[0], volume_usd: null }],
    [{ ...series.series[0], volume_usd: '0' }]]) {
    await assert.rejects(getDailyVolume(ts, day, get(daily, { ...series, series: rows })))
  }
})
test('malformed and negative USD never use the legacy-null fallback', async () => {
  for (const volume_usd of ['', 'garbage', '-1', {}, true, 'Infinity', undefined]) {
    await assert.rejects(getDailyVolume(ts, day, get({ ...daily, volume_usd })))
  }
})
test('wrong identities, clocks, catalogs and invalid counts reject', async () => {
  for (const feed of [{ ...daily, timestamp: ts + 86400 }, { ...daily, date: '2026-09-23' },
    { ...daily, trade_count: '28' }, { ...daily, trade_count: -1 },
    { ...daily, trade_count: 1.5 }, { ...daily, volume_usd: '0' }]) {
    await assert.rejects(getDailyVolume(ts, day, get(feed)))
  }
  for (const patch of [{ grain: 'hourly' }, { timezone: 'REDACTED' },
    { methodology: 'listing' }, { series: {} }]) {
    await assert.rejects(getDailyVolume(ts, day, get(daily, { ...series, ...patch })))
  }
})
test('HTTP failure propagates without publishing zero', async () => {
  await assert.rejects(getDailyVolume(ts, day, async () => { throw new Error('HTTP 503') }), /503/)
})
