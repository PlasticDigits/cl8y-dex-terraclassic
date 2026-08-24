#!/usr/bin/env bash
# Shared cl8y_dex wasm freshness checks (source vs artifact, artifact vs deploy stamp).

# Sets STALE_CONTRACTS to basenames of wasm older than contract sources.
# Returns 0 when all artifacts are fresh, 1 when any are stale.
dex_wasm_stale_vs_sources() {
  local repo_root="$1"
  local artifacts_dir="${repo_root}/smartcontracts/artifacts"
  local contracts_dir="${repo_root}/smartcontracts/contracts"
  local wasm basename short short_dash src_dir newest_src

  STALE_CONTRACTS=()
  for wasm in "$artifacts_dir"/cl8y_dex_*.wasm; do
    [ -f "$wasm" ] || continue
    basename=$(basename "$wasm" .wasm)
    short=${basename#cl8y_dex_}
    short_dash=${short//_/-}
    if [ -d "$contracts_dir/$short_dash" ]; then
      src_dir="$contracts_dir/$short_dash"
    elif [ -d "$contracts_dir/hooks/$short_dash" ]; then
      src_dir="$contracts_dir/hooks/$short_dash"
    else
      continue
    fi
    newest_src=$(find "$src_dir/src" -name '*.rs' -newer "$wasm" 2>/dev/null | head -1)
    if [ -n "$newest_src" ]; then
      STALE_CONTRACTS+=("$basename")
    fi
  done
  # GitLab #620 — LocalTerra tax seed uses these artifacts; stale wasm must fail deploy.
  local ctax_pair
  for ctax_pair in \
    "cl8y_community_tax_token:community-tax-token" \
    "cl8y_community_token_launcher:community-token-launcher" \
    "cl8y_community_tax_autolp:community-tax-autolp"; do
    basename="${ctax_pair%%:*}"
    src_dir="$contracts_dir/${ctax_pair#*:}"
    wasm="$artifacts_dir/${basename}.wasm"
    [ -f "$wasm" ] || continue
    [ -d "$src_dir/src" ] || continue
    newest_src=$(find "$src_dir/src" -name '*.rs' -newer "$wasm" 2>/dev/null | head -1)
    if [ -n "$newest_src" ]; then
      STALE_CONTRACTS+=("$basename")
    fi
  done
  [ "${#STALE_CONTRACTS[@]}" -eq 0 ]
}

# Returns 0 when any cl8y_dex artifact is newer than the deploy stamp (rebuild without redeploy).
dex_wasm_newer_than_stamp() {
  local repo_root="$1"
  local stamp="$2"
  local artifacts_dir="${repo_root}/smartcontracts/artifacts"
  [ -n "$(find "$artifacts_dir" -maxdepth 1 \( -name 'cl8y_dex_*.wasm' -o -name 'cl8y_community_*.wasm' \) -newer "$stamp" -print -quit 2>/dev/null)" ]
}
