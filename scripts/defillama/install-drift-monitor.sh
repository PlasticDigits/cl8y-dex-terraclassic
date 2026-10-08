#!/usr/bin/env bash
# Install a user-owned, read-only daily drift check; credentials are unnecessary.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# Keep unit values literal and refuse characters interpreted by systemd.
[[ "$root" =~ ^/[a-zA-Z0-9_./-]+$ ]] || { echo 'Unsupported checkout path for systemd unit' >&2; exit 2; }
units="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$units" "${XDG_STATE_HOME:-$HOME/.local/state}/cl8y-defillama"
cat > "$units/cl8y-defillama-drift.service" <<EOF
[Unit]
Description=CL8Y DeFiLlama completed UTC-day drift (read only)

[Service]
Type=oneshot
WorkingDirectory=$root
ExecStart=/usr/bin/python3 $root/scripts/qa/reconcile-defillama-1354.py --lookback-days 90 --output %S/cl8y-defillama/drift.json
TimeoutStartSec=10min
NoNewPrivileges=true
PrivateTmp=true
StandardOutput=journal
StandardError=journal
EOF
cat > "$units/cl8y-defillama-drift.timer" <<'EOF'
[Unit]
Description=Daily CL8Y DeFiLlama drift check after UTC-day close

[Timer]
OnCalendar=*-*-* 01:15:00 UTC
RandomizedDelaySec=5min
Persistent=true
Unit=cl8y-defillama-drift.service

[Install]
WantedBy=timers.target
EOF
systemd-analyze --user verify "$units/cl8y-defillama-drift.service" "$units/cl8y-defillama-drift.timer"
systemctl --user daemon-reload
systemctl --user enable --now cl8y-defillama-drift.timer
systemctl --user list-timers cl8y-defillama-drift.timer --no-pager
printf 'Findings exit 1; unavailable data exits 2. Inspect drift.json and journalctl --user -u cl8y-defillama-drift.service.\n'
