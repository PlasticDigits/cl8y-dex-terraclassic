const { test } = require('node:test');
const assert = require('node:assert/strict');
const { PAIR, PAIR_B, PREFIX, REQUEST_CAP, market, resource, row } = require('./browser-price-races-1512.cjs');
test('race fixture substitutes only two exact known pair price resources', () => {
  for (const address of [PAIR, PAIR_B]) {
    for (const [suffix, kind] of [['', 'pair'], ['/trades', 'trades'], ['/candles', 'candles']])
      assert.deepEqual(resource(new URL(`https://indexer.dex.cl8y.com/api/v1/pairs/${address}${suffix}`)), { address, kind });
    assert.equal(resource(new URL(`https://indexer.dex.cl8y.com/api/v1/pairs/${address}/stats`)), null);
  }
  assert.equal(resource(new URL('https://indexer.dex.cl8y.com/api/v1/pairs/foreign/trades')), null);
  assert.throws(() => market('foreign'), /UnknownFixturePair/);
});
test('independent scalar fixtures distinguish both markets and late indexed update without actors', () => {
  assert.equal(Number(row(PAIR).price_usd) / Number(row(PAIR).price), 3);
  assert.equal(Number(row(PAIR_B).price_usd) / Number(row(PAIR_B).price), 10);
  assert.equal(Number(row(PAIR_B, true).price_usd) / Number(row(PAIR_B, true).price), 15);
  for (const address of [PAIR, PAIR_B]) {
    assert.equal(row(address).sender, '');
    assert.equal(row(address).tx_hash, '');
    assert.equal(row(address).pair_address, address);
  }
  assert.notEqual(PREFIX.trade, PREFIX.charts);
  assert.equal(REQUEST_CAP, 240);
});
