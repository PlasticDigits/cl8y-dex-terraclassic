#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
manifest=$(mktemp)
trap 'rm -f "$manifest"' EXIT
cd "$repo/indexer"
# Compile separately: the59s requirement applies to each executed test process.
cargo test --locked --lib --test route_local_quotes_1452 --no-run --message-format=json > "$manifest"
mapfile -t binaries < <(python3 - "$manifest" <<'PY'
import json,sys
found={}
for line in open(sys.argv[1]):
 try: row=json.loads(line)
 except ValueError:continue
 target=row.get('target',{});path=row.get('executable')
 if row.get('reason')=='compiler-artifact' and path:
  if target.get('name')=='cl8y_dex_indexer' and 'lib' in target.get('kind',[]):found['lib']=path
  if target.get('name')=='route_local_quotes_1452':found['http']=path
assert set(found)=={'lib','http'},'missing focused test executables'
print(found['lib']);print(found['http'])
PY
)
[[ ${#binaries[@]} == 2 ]]
timeout --signal=TERM --kill-after=2s 59s "${binaries[0]}" api::rolling_readiness::tests --test-threads=1
timeout --signal=TERM --kill-after=2s 59s "${binaries[1]}" api_rolling_readiness --test-threads=1
