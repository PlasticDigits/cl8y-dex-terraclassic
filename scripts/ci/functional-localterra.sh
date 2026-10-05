#!/usr/bin/env bash
# Explicit scheduled policy (#1400): run on a dedicated disposable LocalTerra
# host at 03:00 UTC daily, and before a release. This is never a PR job.
# The scheduler owns provisioning and failure triage; see docs/qa/issue-1400.
# Invoke: timeout --signal=TERM --kill-after=30s 45m bash scripts/ci/functional-localterra.sh
set -euo pipefail
[[ "${CI_PIPELINE_EVENT:-manual}" != pull_request ]] || {
  echo 'LocalTerra signing policy must not run as an untrusted PR job' >&2; exit 1;
}
root="${CI_WORKSPACE:-$PWD}"
cd "$root"
[[ -f frontend-dapp/.env.local && -f indexer/.env ]] || {
  echo 'Provision the dedicated disposable LocalTerra host first' >&2; exit 1;
}
make has-localterra
curl -fsS --max-time 5 http://127.0.0.1:3001/api/v1/overview >/dev/null
optimizer_image="$(python3 - <<'PY'
import json
from pathlib import Path
name = "cosmwasm/workspace-optimizer:0.16.1"
print(name + "@" + json.loads(Path("scripts/ci/image-pins.json").read_text())[name])
PY
)"
python3 scripts/ci/build-provenance.py --root "$root" \
  --verify smartcontracts/artifacts/build-provenance.json --expected-image "$optimizer_image"
receipt_dir="$(mktemp -d "${TMPDIR:-/tmp}/cl8y-localterra1400.XXXXXXXX")"
container="${LOCALTERRA_CONTAINER:-cl8y-dex-terraclassic-localterra-1}"
lcd_url="$(python3 - <<'PY'
import json
import os
from pathlib import Path
from urllib.parse import urlparse
from urllib.request import urlopen

public = {}
for line in Path("frontend-dapp/.env.local").read_text().splitlines():
    key, sep, value = line.partition("=")
    if sep and key == "VITE_TERRA_LCD_URL":
        public[key] = value.strip().strip("\"'")
url = public["VITE_TERRA_LCD_URL"].rstrip("/")
parsed = urlparse(url)
assert parsed.hostname in ("127.0.0.1", "localhost", "::1"), "browser LCD must be local"
assert parsed.scheme in ("http", "https") and not parsed.username and not parsed.password
for name in ("VITE_TERRA_LCD_URL", "E2E_LCD_URL", "E2E_LCD_PROXY_URL", "PLAYWRIGHT_BASE_URL"):
    assert not os.environ.get(name), f"unexpected browser endpoint override: {name}"
with urlopen(url + "/cosmos/base/tendermint/v1beta1/node_info", timeout=5) as response:
    assert json.load(response)["default_node_info"]["network"] == "localterra", "browser chain must be localterra"
print(url)
PY
)"

# The harness checks chain_id=localterra, signs with the test keyring, creates
# isolated contracts/accounts, and compares balances/sinks/supply after failures.
# It does not reset another worktree's chain or shared DB volumes.
python3 scripts/qa/localterra-route-tax-1381.py --container "$container" \
  --evidence-file "$receipt_dir/funding.json"
python3 scripts/qa/localterra-route-tax-flows-1381.py --container "$container" \
  --fixture "$receipt_dir/funding.json" \
  --stack-env frontend-dapp/.env.local --evidence-file "$receipt_dir/atomic-flows.json"

# One funded browser signing/recovery case, plus hybrid fill and limit escrow
# place/cancel. Strict chain mode and zero retries keep failures visible/bounded.
CI=1 REQUIRE_LOCALTERRA=1 PLAYWRIGHT_SKIP_CHAIN=0 PLAYWRIGHT_WEB_PORT=3173 VITE_TERRA_LCD_URL="$lcd_url" \
  PLAYWRIGHT_JSON_OUTPUT_FILE="$receipt_dir/browser-private.json" \
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test \
  --project=e2e-tx --workers=1 --retries=0 --reporter=json \
  --grep 'Simulated Wallet swap shows recovery UX|hybrid swap emits wasm limit_order_fill|place limit shows success with tx hash|cancel limit via my open limits panel' \
  e2e/terra-broadcast-recovery.spec.ts e2e/hybrid-swap.spec.ts e2e/limit-orders-tx.spec.ts \
  > "$receipt_dir/browser-console-private.log" 2>&1

# Publish only a reduced synthetic receipt; raw browser traces/reports remain
# local. Scheduled checks have no skip-to-PASS semantics.
python3 - "$receipt_dir" "$(git rev-parse HEAD)" <<'PY'
from datetime import datetime, timezone
import json
from pathlib import Path
import sys

folder, revision = Path(sys.argv[1]), sys.argv[2]
for filename in ("funding.json", "atomic-flows.json"):
    record = json.loads((folder / filename).read_text())
    assert record["status"] == "PASS", f"incomplete LocalTerra result: {filename}"
browser = json.loads((folder / "browser-private.json").read_text())
stats = browser["stats"]
assert stats["expected"] >= 4, "critical browser signing cases missing"
assert stats["unexpected"] == stats["flaky"] == stats["skipped"] == 0, stats
receipt = {
    "checkout_revision": revision, "utc": datetime.now(timezone.utc).isoformat(),
    "scope": "synthetic-localterra-only", "status": "PASS",
    "browser_stack_revision": "unverified-preprovisioned-stack",
    "browser_scope": "funded signing behavior; source/artifact alignment requires scheduler deployment evidence",
    "browser": {key: stats[key] for key in ("expected", "unexpected", "flaky", "skipped")},
    "fixture_artifacts": json.loads((folder / "funding.json").read_text())["codes"],
    "artifact_source_verification": "receipt source/lock/image/output verified; independent rebuild not performed here",
}
(folder / "scheduled-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
print("PASS: scheduled LocalTerra signing/funding/rollback; receipt " + str(folder / "scheduled-receipt.json"))
PY
