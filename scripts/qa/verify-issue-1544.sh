#!/usr/bin/env bash
# Focused source/config verification only; live acceptance is recorded separately.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ROOT_DIR="$PWD"
run() {
  printf '%s\n' "$1"
  shift
  timeout --signal=TERM --kill-after=2s 55s "$@"
}
run 'Monitoring exporter and persistent alert receipt boundaries' python3 -m unittest discover -s monitoring/dex -p test_exporter.py
run 'Monitoring-only drill state machine and delivery proof' python3 -m unittest discover -s monitoring/dex -p test_drill.py
run 'Private S3 candidate retention, safe failure diagnostics and resume' python3 scripts/ops/test-stage-logs-s3-credentials.py
run 'Real terminal hidden prompts with noninteractive guarded child' python3 scripts/ops/test_s3_guarded_operator.py
run 'Loki-only replacement identity, rollback and real storage proof boundaries' python3 scripts/ops/test_replace_loki_s3_host.py
run 'One-shot maintenance worker identity, isolation and private receipts' python3 scripts/ops/test_loki_s3_adoption_worker.py
run 'Private adoption journal and scoped Coolify deployment controls' python3 scripts/ops/test_adopt_logs_s3.py
run 'Collector credential-name inventory including digits' python3 scripts/ops/test_logs_collector_inventory.py
run 'Scoped ops fake-API mutation and diagnostic guards' python3 scripts/qa/test-dex-monitoring-ops-1544.py
run 'Exact target identity, freshness and Grafana response validation' python3 scripts/qa/test-swap-monitoring-1479-1480.py
run 'Route monitoring uses scoped Prometheus and preserves failure classification' python3 scripts/qa/test-probe-route-monitoring-1489.py
run 'Native monitoring fixed queries and hard 19-read budget' python3 scripts/qa/test-native-recovery-1505.py \
  Tests.test_monitoring_commit_queries_match_actual_hub_producer \
  Tests.test_monitoring_uses_bounded_fixed_queries_without_raw_lines \
  Tests.test_native_monitoring_optional_branches_use_the_real_reader_budget \
  Tests.test_native_monitoring_cli_retains_old_cap_failure_and_uses_explicit_profile \
  Tests.test_monitoring_budget_profile_is_closed_before_credentials_or_transport
run 'Monitoring Compose configuration (no lifecycle operation)' docker compose --project-directory "$ROOT_DIR" -f monitoring/dex/compose.yaml config --quiet
PROM_IMAGE='prom/prometheus:v3.5.0@sha256:63805ebb8d2b3920190daf1cb14a60871b16fd38bed42b857a3182bc621f4996'
ALERT_IMAGE='prom/alertmanager:v0.28.1@sha256:27c475db5fb156cab31d5c18a4251ac7ed567746a2483ff264516437a39b15ba'
run 'Prometheus config/rule parser in isolated disposable container' docker run --rm --network none --memory 128m --cpus 0.25 \
  --entrypoint /bin/promtool --mount "type=bind,src=$ROOT_DIR/monitoring/dex,dst=/etc/prometheus,readonly" \
  "$PROM_IMAGE" check config /etc/prometheus/prometheus.yml
run 'Prometheus rule behavior with synthetic availability/lag/drill samples' docker run --rm --network none --memory 128m --cpus 0.25 \
  --entrypoint /bin/promtool --mount "type=bind,src=$ROOT_DIR/monitoring/dex,dst=/etc/prometheus,readonly" \
  "$PROM_IMAGE" test rules /etc/prometheus/alerts.test.yml
run 'Alertmanager receiver parser in isolated disposable container' docker run --rm --network none --memory 128m --cpus 0.25 \
  --entrypoint /bin/amtool --mount "type=bind,src=$ROOT_DIR/monitoring/dex,dst=/etc/alertmanager,readonly" \
  "$ALERT_IMAGE" check-config /etc/alertmanager/alertmanager.yml
run 'Launch monitoring crosslinks and dashboard JSON' python3 scripts/check_launch_monitoring_docs.py
run 'Dashboard JSON parses' python3 -m json.tool monitoring/dex/dashboard.json /dev/null
run 'Verifier shell syntax' bash -n scripts/qa/verify-issue-1544.sh
printf '%s\n' 'PASS: focused source/config checks. Live deployment, replacement persistence, Loki/Grafana queries and drill require separate actual evidence.'
