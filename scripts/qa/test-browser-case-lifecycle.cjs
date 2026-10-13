const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const lifecyclePath = path.join(__dirname, "browser-case-lifecycle.cjs");
const { recordThenClearInput, preserveFailureClass, countBoundedRead } =
  fs.existsSync(lifecyclePath) ? require(lifecyclePath) : {};

test("the full adoption observation is recorded before clearing the original input", async () => {
  const events = [];
  let complete;
  const observed = new Promise((resolve) => {
    complete = resolve;
  });
  const pending = recordThenClearInput(
    { fill: async (value) => events.push(["input", value]) },
    async () => {
      events.push(["observation", "started"]);
      await observed;
      events.push(["observation", "recorded"]);
      return { adopted: true };
    },
  );
  assert.deepEqual(events, [["observation", "started"]]);
  complete();
  assert.deepEqual(await pending, { adopted: true });
  assert.deepEqual(events, [
    ["observation", "started"],
    ["observation", "recorded"],
    ["input", "0"],
  ]);
});

test("failed observation keeps its original failure and is never cleared into a passing sample", async () => {
  const error = new Error("ReadBudget");
  let cleared = false;
  await assert.rejects(
    recordThenClearInput(
      {
        fill: async () => {
          cleared = true;
        },
      },
      async () => {
        throw error;
      },
    ),
    (value) => value === error,
  );
  assert.equal(cleared, false);
});

test("specific cap failure survives the induced downstream timeout", () => {
  assert.equal(
    preserveFailureClass("ReadBudget", "TimeoutError"),
    "ReadBudget",
  );
  assert.equal(preserveFailureClass(undefined, "TimeoutError"), "TimeoutError");
});

test("attempts remain distinct from admitted reads and never raise the 100 or 600 cap", () => {
  for (const cap of [100, 600]) {
    let attempted = 0,
      admitted = 0;
    for (let i = 0; i < cap + 3; i++) {
      const budget = countBoundedRead(attempted, cap);
      attempted = budget.attempted;
      if (budget.allowed) admitted++;
    }
    assert.equal(admitted, cap);
    assert.equal(attempted, cap + 3);
  }
});

test("request diagnostics remain fixed families without actor identities or paths", () => {
  const { recordRequestDiagnostic } = require(lifecyclePath);
  const families = {};
  recordRequestDiagnostic(families, "/api/v1/route/solve/progress", true);
  recordRequestDiagnostic(families, "/api/v1/route/solve/progress", false);
  recordRequestDiagnostic(
    families,
    "/api/v1/tokens/private-marker/pairs",
    true,
  );
  for (let i = 0; i < 1000; i++)
    recordRequestDiagnostic(families, `/unknown/private-marker/${i}`, false);
  assert.deepEqual(families.route_progress, {
    attempted: 2,
    budget_admitted: 1,
  });
  assert.deepEqual(families.token_data, { attempted: 1, budget_admitted: 1 });
  assert.equal(Object.keys(families).length, 3);
  assert.equal(JSON.stringify(families).includes("private-marker"), false);
});
