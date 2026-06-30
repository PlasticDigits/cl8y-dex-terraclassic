# Runbook: Emergency pause and trading blacklist commands

Copy-pastable **`terrad tx wasm execute`** recipes for governance emergency controls on the **factory** contract. Use under time pressure during an active incident ([SEC-B11](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/399), GitLab **#399**).

**Related:** [Security model § Trading blacklist](../security-model.md#trading-blacklist-compliance--incident-response), [ADR 0003](../adr/0003-governance-trading-blacklist.md), [user incident FAQ](../user-incident-faq.md), [incident triage template](../templates/incident-dex-indexer.md). Agent playbook: [`skills/AGENTS_EMERGENCY_COMMANDS.md`](../../skills/AGENTS_EMERGENCY_COMMANDS.md).

---

## Before you broadcast

1. **Confirm governance key** — only the factory `governance` address may execute these messages ([`get_config`](../deployment-guide.md)).
2. **Fill parameters** — export the variables in [Environment](#environment) (or replace inline).
3. **Dry-run on staging** — rehearse on LocalTerra: `make verify-issue-399` (requires `make deploy-local`).
4. **Record tx hashes** — paste into the [incident timeline](../templates/incident-dex-indexer.md#incident-timeline) table for audit trail.

---

## Environment

Set once per session (mainnet example):

```bash
export FACTORY_ADDR="<factory_contract_addr>"
export GOVERNANCE_KEY="<governance_key_name>"   # keyring entry for factory governance
export CHAIN_ID="columbus-5"
export NODE="https://terra-classic-rpc.publicnode.com:443"
export LCD="https://terra-classic-lcd.publicnode.com"

# Per-operation targets (fill before each block below)
export PAIR_ADDR="<pair_contract_addr>"
export WALLET_ADDR="<wallet_to_block>"
export TOKEN_ADDR="<cw20_token_contract_addr>"
```

**LocalTerra rehearsal** (after `make deploy-local`):

```bash
source indexer/.env
export FACTORY_ADDR="$FACTORY_ADDRESS"
export GOVERNANCE_KEY="test1"
export CHAIN_ID="localterra"
export NODE="http://127.0.0.1:26657"
export LCD="http://127.0.0.1:1317"
# PAIR_ADDR / TOKEN_ADDR — pick from factory pairs query or frontend .env.local
```

Shared tx flags (append to every command):

```bash
--from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto \
  --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

---

## 1. Pause a pair

Stops swaps, liquidity changes, limit placement/cancel/claim, and book clean on **one** pair ([invariant **L6**](../contracts-security-audit.md)).

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{set_pair_paused:{pair:$pair,paused:true}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:**

```bash
terrad query wasm contract-state smart "$PAIR_ADDR" '{"is_paused":{}}' \
  --node "$LCD" | jq '.data'
# Expect: {"paused":true}
```

---

## 2. Unpause a pair

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{set_pair_paused:{pair:$pair,paused:false}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:**

```bash
terrad query wasm contract-state smart "$PAIR_ADDR" '{"is_paused":{}}' \
  --node "$LCD" | jq '.data'
# Expect: {"paused":false}
```

---

## 3. Blacklist a wallet

Blocks the address on **all** pairs and router paths ([ADR 0003](../adr/0003-governance-trading-blacklist.md)).

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg address "$WALLET_ADDR" \
  '{blacklist_wallet:{address:$address}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:**

```bash
terrad query wasm contract-state smart "$FACTORY_ADDR" "$(jq -nc \
  --arg wallet "$WALLET_ADDR" \
  --arg pair "$PAIR_ADDR" \
  --argjson tokens "[\"$TOKEN_ADDR\"]" \
  '{blacklist_check:{wallet:$wallet,tokens:$tokens,pair:$pair,pairs:[]}}')" \
  --node "$LCD" | jq '.data'
# Expect: blocked=true, wallet_blacklisted=true
```

---

## 4. Unblacklist a wallet

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg address "$WALLET_ADDR" \
  '{unblacklist_wallet:{address:$address}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:** repeat the `blacklist_check` query above — expect `wallet_blacklisted=false` and `blocked=false` (when token/pair are also clear).

---

## 5. Blacklist a token

Blocks any trade touching the CW20 on any pair.

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg token "$TOKEN_ADDR" \
  '{blacklist_token:{token:$token}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:**

```bash
terrad query wasm contract-state smart "$FACTORY_ADDR" "$(jq -nc \
  --argjson tokens "[\"$TOKEN_ADDR\"]" \
  '{blacklist_check:{wallet:null,tokens:$tokens,pair:null,pairs:[]}}')" \
  --node "$LCD" | jq '.data'
# Expect: blocked=true; blacklisted_tokens includes TOKEN_ADDR
```

---

## 6. Unblacklist a token

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg token "$TOKEN_ADDR" \
  '{unblacklist_token:{token:$token}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:** repeat token `blacklist_check` — `blacklisted_tokens` empty and `blocked=false`.

---

## 7. Blacklist a pair

Blocks all protocol actions on the registered pair contract.

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{blacklist_pair:{pair:$pair}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:**

```bash
terrad query wasm contract-state smart "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{blacklist_check:{wallet:null,tokens:[],pair:$pair,pairs:[]}}')" \
  --node "$LCD" | jq '.data'
# Expect: blocked=true, pair_blacklisted=true
```

---

## 8. Unblacklist a pair

```bash
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{unblacklist_pair:{pair:$pair}}')" \
  --from "$GOVERNANCE_KEY" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna \
  -y
```

**Confirm:** repeat pair `blacklist_check` — `pair_blacklisted=false` and `blocked=false`.

---

## LocalTerra rehearsal evidence

Automated rehearsal (all eight operations + post-tx queries):

```bash
make verify-issue-399
# or: ./scripts/qa/verify-issue-399.sh
```

Requires LocalTerra up and `make deploy-local` env (`indexer/.env` + `frontend-dapp/.env.local`). The script is idempotent: it pauses/unpauses and blacklists/unblacklists test fixtures, then restores the chain to the pre-rehearsal state.

Doc invariant (no chain required):

```bash
make check-emergency-commands-docs
```

---

## Incident workflow

During triage, open the [incident template](../templates/incident-dex-indexer.md) **Mitigation** section — it links here for on-chain factory controls. Pair pause is appropriate for pool-specific exploits; wallet/token/pair blacklist for compliance or broader trading halts. See [user incident FAQ](../user-incident-faq.md) for trader-facing impact.
