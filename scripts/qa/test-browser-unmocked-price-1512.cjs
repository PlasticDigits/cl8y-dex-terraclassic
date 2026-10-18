const { test } = require('node:test');
const assert = require('node:assert/strict');
const { projectTrade, projectCandles, plottable, matchesScalar } = require('./browser-unmocked-price-1512.cjs');
const { PAIR } = require('./browser-price-campaign-1512.cjs');
test('actual response projection drops actor/tape linkage and does not invent missing USD', () => {
  const row = { pair_address: PAIR, block_timestamp: '2026-01-01T00:00:00Z', price: '160',
    price_usd: null, sender: 'private-fixture-actor', tx_hash: 'private-fixture-transaction' };
  const projected = projectTrade([row], Date.parse('2026-01-02T00:00:00Z'));
  assert.equal(projected.usd, null);
  assert.equal(projected.human, 160);
  assert(!JSON.stringify(projected).includes('private-fixture'));
  assert.equal(matchesScalar('—', projected.usd), true);
  assert.equal(matchesScalar('160', projected.usd), false);
  const valid = projectTrade([{ ...row, price_usd: '0.8' }], Date.parse('2026-01-02T00:00:00Z'));
  assert.equal(matchesScalar('0.005', valid.usd / valid.human), true);
  assert.equal(matchesScalar('5e9', valid.usd / valid.human), false);
});
test('historical plot availability follows stored USD leg and required human conversion', () => {
  const base = { open_time: '2026-01-01T00:00:00Z', open: '1', high: '2', low: '0.5', close: '1.5' };
  const rows = projectCandles([base, { ...base, usd_leg: 'asset_1', open: null,
    subject_open: '1', subject_high: '2', subject_low: '0.5', subject_close: '1.5' }]);
  assert.equal(plottable(rows, false), 1);
  assert.equal(plottable(rows, true), 1);
  assert(!JSON.stringify(rows).includes('2026-01-01'));
});
