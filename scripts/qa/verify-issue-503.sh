#!/usr/bin/env bash
# Automated verification for GitLab #503 — UST1/wrap production ops hardening.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
PASS=0; FAIL=0; declare -a RESULTS=()
ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
run_step() { local label="$1"; shift; echo ""; echo "[$label]"; if "$@"; then ok "$label"; else bad "$label"; fi; }
echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #503 — UST1/wrap production ops"
echo "════════════════════════════════════════════════════════════════"
run_step "registry pack exists (no secrets)" \
  bash -c 'test -f deployments/mainnet-ust1-wrap/REGISTRY.md \
    && test -f deployments/mainnet-ust1-wrap/coolify.env.example \
    && test -f deployments/mainnet-ust1-wrap/README.md \
    && grep -q "terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2" deployments/mainnet-ust1-wrap/REGISTRY.md \
    && grep -q "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2" deployments/mainnet-ust1-wrap/REGISTRY.md \
    && grep -q "terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l" deployments/mainnet-ust1-wrap/REGISTRY.md \
    && grep -q "VITE_UST1_WINDOW_ADDRESS" deployments/mainnet-ust1-wrap/coolify.env.example \
    && grep -q "VITE_WRAP_MAPPER_ADDRESS" deployments/mainnet-ust1-wrap/coolify.env.example \
    && ! grep -Eiq "BEGIN (RSA |EC )?PRIVATE|mnemonic=|\"mnemonic\"" deployments/mainnet-ust1-wrap/*'
run_step "runbook + pause playbook + skill (O1–O8)" \
  bash -c 'test -f docs/runbooks/ust1-wrap-production-ops.md \
    && test -f docs/runbooks/wrap-mapper-pause.md \
    && test -f skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md \
    && for id in O1 O2 O3 O4 O5 O6 O7 O8; do
         grep -q "\*\*${id}\*\*" docs/runbooks/ust1-wrap-production-ops.md || exit 1
         grep -q "$id" skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md || exit 1
       done \
    && grep -q "set_paused" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "set_wrapping_paused" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "ust1-oracle" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "ORACLE_ADDR\|set_paused.*oracle\|oracle.*set_paused\|A2. ust1-oracle" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "rate_limit" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "set_rate_limit\|SetRateLimit" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "set_paused" docs/runbooks/wrap-mapper-pause.md \
    && grep -q "verify_oracle_operator_env" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "ORACLE_MAX_SILENCE" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "21600" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "21600" scripts/lib/ust1-wrap-ops-defaults.sh \
    && grep -q "Oracle bot operator" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "Treasury / wrap governance" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "Treasury execute is" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -qi "agents must not broadcast" docs/runbooks/ust1-wrap-production-ops.md \
    && grep -q "ust1-wrap-production-ops\|#503\|check-ust1-wrap-ops-health" docs/runbooks/launch-checklist.md \
    && grep -q "UST1_OPS_STRICT_INVENTORY\|STRICT_INVENTORY" scripts/check-ust1-wrap-ops-health.sh \
    && grep -q "refusing to treat as 0\|LCD bank/supply query empty" scripts/check-ust1-wrap-ops-health.sh'
run_step "phase cross-links #502/#506/#507/#508 + #503" \
  bash -c 'for n in 502 503 506 507 508; do grep -q "#${n}" docs/runbooks/ust1-wrap-production-ops.md || exit 1; done
    grep -q "ust1-window" docs/runbooks/ust1-wrap-production-ops.md
    grep -q "ustr-cmm" docs/runbooks/ust1-wrap-production-ops.md'
run_step "AGENTS.md + docs index + soft-launch crosslink" \
  bash -c 'grep -q "AGENTS_UST1_WRAP_PRODUCTION_OPS" AGENTS.md \
    && grep -q "ust1-wrap-production-ops" docs/README.md \
    && grep -q "ust1-wrap-production-ops\|#503\|mainnet-ust1-wrap" docs/runbooks/mainnet-soft-launch.md \
    && grep -q "#503" skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md \
    && grep -q "#503" skills/AGENTS_UST1_WINDOW_UI.md \
    && grep -q "mainnet-ust1-wrap\|REGISTRY.md\|AGENTS_UST1_WRAP_PRODUCTION_OPS" skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md'
run_step "FAQ covers UST1 + wrap pause; checker markers" \
  bash -c 'grep -q "UST1 oracle window" docs/user-incident-faq.md \
    && grep -q "Wrap pause" docs/user-incident-faq.md \
    && grep -q "wrap-mapper" docs/user-incident-faq.md \
    && grep -q "UST1 oracle window" scripts/check_user_incident_faq_docs.py \
    && python3 scripts/check_user_incident_faq_docs.py'
run_step "scripts executable + shell syntax" \
  bash -c 'chmod +x scripts/check-ust1-wrap-ops-health.sh scripts/qa/verify-issue-503.sh scripts/lib/ust1-wrap-ops-defaults.sh \
    && bash -n scripts/check-ust1-wrap-ops-health.sh \
    && bash -n scripts/qa/verify-issue-503.sh \
    && bash -n scripts/lib/ust1-wrap-ops-defaults.sh \
    && grep -q "check-ust1-wrap-ops-health" Makefile \
    && grep -q "verify-issue-503" Makefile \
    && grep -q "UST1_OPS_WINDOW" scripts/lib/ust1-wrap-ops-defaults.sh \
    && grep -q "effective_swap" scripts/check-ust1-wrap-ops-health.sh \
    && grep -q "total_supply" scripts/check-ust1-wrap-ops-health.sh'
run_step "QA template documents columbus-5 wrap pause smoke" \
  bash -c 'grep -q "columbus-5\|Columbus-5" docs/qa-templates/wrap-unwrap-test-pass.md \
    && grep -q "#503" docs/qa-templates/wrap-unwrap-test-pass.md \
    && grep -q "wrap-mapper-pause.md" docs/qa-templates/wrap-unwrap-test-pass.md \
    && grep -q "check-ust1-wrap-ops-health" docs/qa-templates/wrap-unwrap-test-pass.md'
if [[ "${VERIFY503_MAINNET:-0}" == "1" ]]; then
  run_step "live mainnet LCD health (VERIFY503_MAINNET=1)" ./scripts/check-ust1-wrap-ops-health.sh
else
  echo ""; echo "[SKIP] live mainnet LCD — set VERIFY503_MAINNET=1 to enable"
fi
echo ""; echo "════════════════════════════════════════════════════════════════"; printf '%s\n' "${RESULTS[@]}"; echo "────────────────────────────────────────────────────────────────"; echo "  PASS: $PASS   FAIL: $FAIL"; echo "════════════════════════════════════════════════════════════════"
[[ "$FAIL" -eq 0 ]] && echo "OK: verify-issue-503" || exit 1
