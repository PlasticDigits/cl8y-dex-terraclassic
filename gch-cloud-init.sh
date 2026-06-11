#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Boot script for cl8y-dex-terraclassic agent VMs — called by cloud-init on each job.
set -euo pipefail

source /etc/gch/job.env
export PATH="/home/agent/.local/bin:$PATH"

# Start local Terra node if project uses docker-compose localterra
# sudo -u agent docker compose -f /home/agent/workspace/docker-compose.localterra.yml up -d

source /home/agent/gch-cloud-init-runner.sh
gch_run_job
