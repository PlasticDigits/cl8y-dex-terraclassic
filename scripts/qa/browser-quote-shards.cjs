// Pure QA corpus selection. No browser, network, wallets or application state.
function directionalGroups(pairs) {
  if (!Array.isArray(pairs) || !pairs.length)
    throw new Error("Empty directional corpus");
  const groups = pairs.flatMap((pair) => {
    if (
      !Array.isArray(pair) ||
      pair.length !== 2 ||
      pair.some(
        (label) =>
          typeof label !== "string" || !/^[A-Za-z0-9_]{1,32}$/.test(label),
      )
    )
      throw new Error("Invalid directional corpus");
    return [pair, [...pair].reverse()].map(([from, to]) => ({
      from,
      to,
      id: `${from}->${to}`,
    }));
  });
  if (new Set(groups.map((group) => group.id)).size !== groups.length)
    throw new Error("Duplicate directional group");
  return groups.map((group, index) => ({ ...group, index }));
}

function parseShard(env, groupCount) {
  const indexText = env.DEX1473_BROWSER_SHARD_INDEX;
  const countText = env.DEX1473_BROWSER_SHARD_COUNT;
  if (indexText === undefined && countText === undefined)
    return { index: 0, count: 1, explicit: false };
  if (
    typeof indexText !== "string" ||
    typeof countText !== "string" ||
    !/^(0|[1-9]\d*)$/.test(indexText) ||
    !/^[1-9]\d*$/.test(countText)
  )
    throw new Error("Both shard index/count must be canonical integers");
  const index = Number(indexText),
    count = Number(countText);
  if (
    !Number.isSafeInteger(index) ||
    !Number.isSafeInteger(count) ||
    count > groupCount ||
    index >= count
  )
    throw new Error("Shard index/count outside directional corpus");
  return { index, count, explicit: true };
}

function selectDirectionalGroups(pairs, env) {
  const corpus = directionalGroups(pairs),
    shard = parseShard(env, corpus.length);
  const selected = corpus.filter(
    (group) => group.index % shard.count === shard.index,
  );
  return {
    corpus,
    selected,
    shard: {
      ...shard,
      directional_group_count: corpus.length,
      selected_directional_group_count: selected.length,
      directional_group_indices: selected.map((group) => group.index),
      directional_group_ids: selected.map((group) => group.id),
    },
  };
}

function amountCases(profile, humanAmounts, decimals) {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 38)
    throw new Error("Invalid corpus decimals");
  const human = humanAmounts.map((value) => ({
    kind: "human_amount",
    human: value,
  }));
  return profile === "full"
    ? [
        {
          kind: "one_raw_unit",
          human: decimals === 0 ? "1" : `0.${"0".repeat(decimals - 1)}1`,
        },
        ...human,
      ]
    : human;
}

function rawAmount(human, decimals) {
  if (typeof human !== "string" || !/^\d+(\.\d+)?$/.test(human))
    throw new Error("Invalid corpus amount");
  const [whole, fraction = ""] = human.split(".");
  if (fraction.length > decimals) throw new Error("Unrepresentable input");
  return (
    BigInt(whole) * 10n ** BigInt(decimals) +
    BigInt(fraction.padEnd(decimals, "0") || "0")
  ).toString();
}

function caseIdentity({
  matrixIdentity,
  profile,
  viewport,
  trial,
  from,
  to,
  spread,
  kind,
  human,
  raw,
}) {
  // JSON tuple preserves distinct one-raw/human cases without delimiter collisions.
  return JSON.stringify([
    matrixIdentity,
    profile,
    viewport,
    trial,
    from,
    to,
    spread,
    kind,
    human,
    raw,
  ]);
}

function expectedCaseIdentities({
  matrixIdentity,
  profile,
  cycles,
  groups,
  assets,
  spreads,
  humanAmounts,
}) {
  const identities = [];
  for (let cycle = 0; cycle < cycles; cycle++)
    for (const viewport of ["desktop", "phone"])
      for (const { from, to } of groups)
        for (const spread of spreads)
          for (const amount of amountCases(
            profile,
            humanAmounts,
            assets[from].decimals,
          ))
            identities.push(
              caseIdentity({
                matrixIdentity,
                profile,
                viewport,
                trial: cycle ? "retest" : "first",
                from,
                to,
                spread,
                ...amount,
                raw: rawAmount(amount.human, assets[from].decimals),
              }),
            );
  return identities;
}

function assertCaseCoverage(expected, observed) {
  const expectedSet = new Set(expected),
    observedSet = new Set(observed);
  if (
    expectedSet.size !== expected.length ||
    observedSet.size !== observed.length ||
    observed.length !== expected.length ||
    observed.some((id) => !expectedSet.has(id))
  )
    throw new Error("Incomplete or duplicate browser case identities");
}

module.exports = {
  directionalGroups,
  parseShard,
  selectDirectionalGroups,
  amountCases,
  rawAmount,
  caseIdentity,
  expectedCaseIdentities,
  assertCaseCoverage,
};
