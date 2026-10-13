const { test } = require('node:test');
const assert = require('node:assert/strict');
const { matchingProgress, intent, displayRaw, sampleFailed } = require('./browser-route-discovery-1489.cjs');
const params = { token_in: 'uluna', token_out: 'terra1output', amount_in: '10000000000',
  max_spread: '0.05', protection_policy: 'quote_relative_v1', pool_only: 'true', max_maker_fills: '8' };
test('exact original native gross intent matches only the actual solve', () => {
  assert.equal(matchingProgress(params, [{ params }]), true);
  assert.equal(matchingProgress({ ...params, token_in: 'terra1clunc' }, [{ params }]), false);
  assert.equal(matchingProgress({ ...params, amount_in: '9800000000' }, [{ params }]), false);
});
test('protection, mode, context and receive directions cannot alias', () => {
  for (const key of ['token_out', 'max_spread', 'protection_policy', 'pool_only', 'hybrid_optimize', 'max_maker_fills', 'discount_bps', 'amount_out']) {
    assert.equal(matchingProgress({ ...params, [key]: 'changed' }, [{ params }]), false, key);
  }
});
test('actor fields never enter public browser evidence', () => {
  assert.throws(() => intent({ ...params, trader: 'private' }), /Actor/);
  assert.throws(() => intent({ ...params, sender: 'private' }), /Actor/);
});
test('progress parity and actual DOM adoption are both mandatory', () => {
  const sample = { useful: true, classified: true, policy_opt_in: true, input_to_dom_ms: 1,
    progress_intent_matches: true, validated_latest_adopted: true };
  assert.equal(sampleFailed(sample), false);
  assert.equal(sampleFailed({ ...sample, progress_intent_matches: false }), true);
  assert.equal(sampleFailed({ ...sample, validated_latest_adopted: false }), true);
});
test('errors/loading cannot be counted as useful and original latency remains bounded', () => {
  const sample = { useful: true, classified: true, policy_opt_in: true, input_to_dom_ms: 749,
    progress_intent_matches: true, validated_latest_adopted: true };
  assert.equal(sampleFailed(sample), false);
  assert.equal(sampleFailed({ ...sample, useful: false }), true);
  assert.equal(sampleFailed({ ...sample, input_to_dom_ms: 751 }), true);
});
test('actual 18-decimal DOM adoption preserves a single raw-unit difference', () => {
  assert.equal(displayRaw('84.648421986431025001', 18), '84648421986431025001');
  assert.notEqual(displayRaw('84.648421986431025001', 18), '84648421986431025000');
  for (const value of ['Infinity', '1e8', '1.0000001', '<script>', '9'.repeat(97)]) {
    assert.equal(displayRaw(value, 6), null);
  }
});
