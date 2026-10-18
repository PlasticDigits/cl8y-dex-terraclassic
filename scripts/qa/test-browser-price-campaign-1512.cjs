const { test } = require('node:test');
const assert = require('node:assert/strict');
const { fixtureKind, trade, candles } = require('./browser-price-campaign-1512.cjs');
const pair = 'terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy';
test('only the known catalog and canonical pair price resources are fixture candidates', () => {
  assert.equal(fixtureKind(new URL('https://indexer.dex.cl8y.com/api/v1/pairs')), 'catalog');
  for (const [suffix, kind] of [['', 'pair'], ['/trades', 'trades'], ['/candles', 'candles']])
    assert.equal(fixtureKind(new URL(`https://indexer.dex.cl8y.com/api/v1/pairs/${pair}${suffix}`)), kind);
  for (const path of ['/api/v1/traders', '/api/v1/route/solve', '/api/v1/pairs/foreign/trades',
    `/api/v1/pairs/${pair}/stats`, `/api/v1/pairs/${pair}/limit-fills`])
    assert.equal(fixtureKind(new URL('https://indexer.dex.cl8y.com' + path)), null);
});
test('synthetic price fixtures contain no actor or transaction linkage and retain separate candle data', () => {
  for (const phase of ['valid', 'missing', 'foreign']) {
    const row = trade(phase);
    assert.equal(row.sender, '');
    assert.equal(row.tx_hash, '');
    assert.equal(row.price, '160.756280899197382191');
    assert.equal(row.price_usd === null, phase === 'missing');
    assert.equal(row.pair_address === pair, phase !== 'foreign');
  }
  const rows = candles();
  assert.equal(rows.length, 2);
  assert(rows.every(row => row.close === '0.85' && row.close_human !== row.close));
});
