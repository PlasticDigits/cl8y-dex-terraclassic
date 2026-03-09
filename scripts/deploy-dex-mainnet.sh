#!/bin/bash
cd "$(dirname "$0")/../smartcontracts"
./scripts/deploy.sh mainnet "$@"
