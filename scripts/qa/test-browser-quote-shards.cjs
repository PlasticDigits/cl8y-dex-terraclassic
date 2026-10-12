#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const {
  selectDirectionalGroups,
  parseShard,
  amountCases,
  expectedCaseIdentities,
  assertCaseCoverage,
} = require("./browser-quote-shards.cjs");
const matrix = JSON.parse(
  fs.readFileSync(path.join(__dirname, "../../docs/qa/issue-1479/matrix.json")),
);
const options = {
  matrixIdentity: "1479-public-matrix",
  profile: "census",
  cycles: 2,
  assets: matrix.assets,
  spreads: matrix.max_spreads,
  humanAmounts: ["0.000001", "0.01", "1", "100", "10000"],
};

test("default unsharded direction order and all 840 census case identities are unchanged", () => {
  const selection = selectDirectionalGroups(matrix.pairs, {});
  assert.equal(selection.corpus.length, 14);
  assert.deepEqual(
    selection.selected.map((group) => [group.from, group.to]),
    matrix.pairs.flatMap((pair) => [pair, [...pair].reverse()]),
  );
  assert.deepEqual(selection.shard.index, 0);
  assert.deepEqual(selection.shard.count, 1);
  assert.equal(selection.shard.explicit, false);
  const cases = expectedCaseIdentities({
    ...options,
    groups: selection.selected,
  });
  assert.equal(cases.length, 840);
  assert.equal(new Set(cases).size, 840);
});

test("two balanced shards cover every viewport/cycle/spread/amount exactly once, including failed inputs", () => {
  const selections = [0, 1].map((index) =>
    selectDirectionalGroups(matrix.pairs, {
      DEX1473_BROWSER_SHARD_INDEX: String(index),
      DEX1473_BROWSER_SHARD_COUNT: "2",
    }),
  );
  const shards = selections.map((s) =>
    expectedCaseIdentities({ ...options, groups: s.selected }),
  );
  assert.deepEqual(
    selections.map((s) => s.selected.length),
    [7, 7],
  );
  assert.deepEqual(
    shards.map((s) => s.length),
    [420, 420],
  );
  assert.equal(shards[0].filter((id) => new Set(shards[1]).has(id)).length, 0);
  const corpus = expectedCaseIdentities({
    ...options,
    groups: selections[0].corpus,
  });
  assertCaseCoverage(corpus, shards.flat());
  const decoded = shards.flat().map((id) => JSON.parse(id));
  assert.deepEqual([...new Set(decoded.map((row) => row[2]))].sort(), [
    "desktop",
    "phone",
  ]);
  assert.deepEqual([...new Set(decoded.map((row) => row[3]))].sort(), [
    "first",
    "retest",
  ]);
  for (const [from, to, spread, human] of [
    ["KENA", "cLUNC", "0.05", "0.01"],
    ["cLUNC", "KENA", "0.01", "0.000001"],
  ]) {
    assert.equal(
      decoded.filter(
        (row) =>
          row[4] === from &&
          row[5] === to &&
          row[6] === spread &&
          row[8] === human,
      ).length,
      4,
    );
  }
});

test("generic shard counts partition the complete corpus without empty or duplicate groups", () => {
  for (const count of [1, 3, 4, 7, 14]) {
    const selections = Array.from({ length: count }, (_, index) =>
      selectDirectionalGroups(matrix.pairs, {
        DEX1473_BROWSER_SHARD_INDEX: String(index),
        DEX1473_BROWSER_SHARD_COUNT: String(count),
      }),
    );
    const lengths = selections.map((s) => s.selected.length);
    assert.ok(lengths.every((length) => length > 0));
    assert.ok(Math.max(...lengths) - Math.min(...lengths) <= 1);
    assertCaseCoverage(
      expectedCaseIdentities({ ...options, groups: selections[0].corpus }),
      selections.flatMap((s) =>
        expectedCaseIdentities({ ...options, groups: s.selected }),
      ),
    );
  }
});

test("partial, noncanonical, unsafe, negative and out-of-range shard options fail closed", () => {
  for (const [index, count] of [
    [undefined, "2"],
    ["0", undefined],
    ["", "2"],
    [" 0", "2"],
    ["01", "2"],
    ["-1", "2"],
    ["0", "0"],
    ["0", "02"],
    ["1.0", "2"],
    ["1e0", "2"],
    ["2", "2"],
    ["0", "15"],
    ["9007199254740992", "2"],
    ["0", "9007199254740992"],
  ]) {
    const env = {};
    if (index !== undefined) env.DEX1473_BROWSER_SHARD_INDEX = index;
    if (count !== undefined) env.DEX1473_BROWSER_SHARD_COUNT = count;
    assert.throws(() => parseShard(env, 14));
  }
});

test("full-profile one-raw and human cases stay distinct even when raw amounts coincide", () => {
  assert.deepEqual(amountCases("full", ["0.000001"], 6), [
    { kind: "one_raw_unit", human: "0.000001" },
    { kind: "human_amount", human: "0.000001" },
  ]);
  const full = expectedCaseIdentities({
    ...options,
    profile: "full",
    humanAmounts: ["0.000001"],
    groups: selectDirectionalGroups([["KENA", "cLUNC"]], {}).selected,
  });
  assert.equal(full.length, 48);
  assert.equal(new Set(full).size, 48);
});

test("completion rejects duplicate, substituted or missing case identities despite equal row counts", () => {
  const expected = ["first", "second"];
  assert.throws(() => assertCaseCoverage(expected, ["first", "first"]));
  assert.throws(() => assertCaseCoverage(expected, ["first", "unknown"]));
  assert.throws(() => assertCaseCoverage(expected, ["first"]));
  assert.doesNotThrow(() => assertCaseCoverage(expected, ["second", "first"]));
  assert.throws(() =>
    selectDirectionalGroups(
      [
        ["KENA", "cLUNC"],
        ["cLUNC", "KENA"],
      ],
      {},
    ),
  );
});
