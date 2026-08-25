'use strict'

const { test } = require('node:test')
const assert = require('node:assert/strict')
const {
  poolReserveAdds,
  nextStartAfter,
  shouldFailPoolErrors,
  paginateComplete,
} = require('./tvlCore')

test('poolReserveAdds uses raw denom/cw20 and skips zeros and LP share', () => {
  const adds = poolReserveAdds({
    assets: [
      { info: { native_token: { denom: 'uluna' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'terra1ust1' } }, amount: '0' },
      { info: { token: { contract_addr: 'terra1ustr' } }, amount: '2500' },
    ],
    total_share: '999999999',
  })
  assert.deepEqual(adds, [
    { key: 'uluna', amount: '1000000' },
    { key: 'terra1ustr', amount: '2500' },
  ])
  assert.ok(!adds.some((a) => a.key === 'total_share' || a.amount === '999999999'))
})

test('pagination continues until a short page', () => {
  const page = [
    { asset_infos: [{ native_token: { denom: 'uluna' } }, { token: { contract_addr: 'a' } }] },
    { asset_infos: [{ native_token: { denom: 'uusd' } }, { token: { contract_addr: 'b' } }] },
  ]
  assert.equal(paginateComplete(page, 30), true)
  assert.equal(paginateComplete(new Array(30).fill(page[0]), 30), false)
  assert.deepEqual(nextStartAfter(page), page[1].asset_infos)
})

test('majority pool-query failure throws (no silent $0)', () => {
  assert.equal(shouldFailPoolErrors(['e1', 'e2', 'e3'], 4), true)
  assert.equal(shouldFailPoolErrors(['e1'], 4), false)
})

test('cLUNC and cUSTC map to uluna and uusd; other CW20s stay raw', () => {
  const { CLUNC, CUSTC } = require('../gems')
  const adds = poolReserveAdds({
    assets: [
      { info: { token: { contract_addr: CLUNC } }, amount: '10' },
      { info: { token: { contract_addr: CUSTC } }, amount: '20' },
      { info: { token: { contract_addr: 'terra1ustr' } }, amount: '30' },
    ],
  })
  assert.deepEqual(adds, [
    { key: 'uluna', amount: '10' },
    { key: 'uusd', amount: '20' },
    { key: 'terra1ustr', amount: '30' },
  ])
})

test('A1: never treat a precomputed USD blob as a pool add', () => {
  const adds = poolReserveAdds({
    liquidity_in_usd: '12345',
    total_liquidity_usd: '999',
    assets: [],
  })
  assert.deepEqual(adds, [])
})
