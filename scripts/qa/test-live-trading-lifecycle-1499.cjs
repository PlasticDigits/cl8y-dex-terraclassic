const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

test("missing exact attestation refuses before any live read and writes a private bounded failure receipt", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "quote-1499-guard-"));
  const output = path.join(directory, "receipt.json");
  try {
    const run = spawnSync(
      process.execPath,
      [path.join(__dirname, "live-trading-lifecycle-1499.cjs")],
      {
        env: {
          ...process.env,
          EXPECTED_SOURCE_SHA: "",
          DEX_LIVE_ATTESTATION: "",
          DEX_LIVE_OUTPUT: output,
        },
        encoding: "utf8",
        timeout: 10000,
      },
    );
    assert.equal(run.status, 1);
    assert.equal(run.stderr, "");
    const receipt = JSON.parse(fs.readFileSync(output, "utf8"));
    assert.equal(receipt.overall, "FAIL");
    assert.equal(receipt.completed, false);
    assert.equal(receipt.read_count, 0);
    assert.equal(receipt.quote_reads, 0);
    assert.equal(receipt.windows.length, 0);
    assert.equal(fs.statSync(output).mode & 0o777, 0o600);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
