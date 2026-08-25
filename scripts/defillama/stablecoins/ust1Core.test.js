'use strict'

const { test } = require('node:test')
const assert = require('node:assert/strict')
const { circulatingFromTokenInfo, mintedBalances } = require('./ust1Core')

test('UST1 circulating is total_supply / 1e6, never $1', () => {
  assert.equal(circulatingFromTokenInfo({ total_supply: '1500000' }), 1.5)
  assert.equal(circulatingFromTokenInfo({ total_supply: '0' }), 0)
  assert.equal(circulatingFromTokenInfo({}), null)
  assert.deepEqual(mintedBalances(1.5), { peggedUSD: 1.5 })
})
