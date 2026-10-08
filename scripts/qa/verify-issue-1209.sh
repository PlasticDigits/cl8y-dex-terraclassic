#!/usr/bin/env bash
# Includes creation-fee parser, replay/API and UI tests plus adjacent invoices.
set -euo pipefail
exec bash "$(dirname "${BASH_SOURCE[0]}")/verify-issue-1210.sh"
