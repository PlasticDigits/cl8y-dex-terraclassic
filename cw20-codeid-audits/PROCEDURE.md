# Standard CW20 code-ID audit procedure

Every new factory whitelist ID follows these steps. Skipping decomp **or** tests is a process fail (**C2**, **C3**). A source-tree review of a **non-matching** rebuild is not the binary audit.

Invariant **F6** (listing pin + write-path re-check) stays. This procedure decides whether a **template** may be **added**. It does not replace F6.

Do **not** add pair / router balance-delta / FoT swap math (**H-01**). A failing suite means **do not whitelist**.

## Steps

1. **Identity** — LCD `CodeInfo`: `code_id`, `data_hash`, creator, instantiate permission, instantiate count. Write `codeids/<id>/meta.json`.
2. **Fetch** — `scripts/fetch-lcd-wasm.sh <id>`. SHA-256 of the wasm bytes **must** equal `data_hash`. Fail closed on mismatch, truncation, or missing ID (**C1**, **G9**). Prefer two LCD endpoints (`LCD_URL` + `LCD_URL_SECONDARY`) and require matching hashes (**G4**). Canonical source is LCD `/cosmwasm/wasm/v1/code/{id}`, not a third-party mirror.
3. **Fingerprint** — `scripts/fingerprint-wasm.sh <id>`. Exports, `producers` / rustc, crate strings (`cw20_base`, `terraport_token`, `tax_map`, `requires_terra`, `ibc_receive`), custom query/execute enums from error strings. Map against [CATALOG.md](CATALOG.md) **D20** / **G8**.
4. **Decompile** — `scripts/decompile-wasm.sh <id>`. Requires `wabt` (`wasm2wat`, `wasm-decompile`, `wasm-objdump`). Missing tools **FAIL** with an install hint; do not skip decomp (**C2**). Store under `codeids/<id>/decomp/`. The inspected artifact is a **decompilation of LCD wasm**, not redistributable source.
5. **Static audit** — walk decomp + fingerprint against every [CATALOG.md](CATALOG.md) row. Record hits, misses, unreadable regions. Classify extra queries (`balance_at`, `total_supply_at`, `tax_map`) as snapshot vs live mutation vs tax.
6. **Automated suite**
   - **Layer A (token-only)** — `cw-multi-test` parameterized tests in `smartcontracts/tests/src/cw20_codeid_harness.rs` against mintable (10184 analogue) and the [mutant library](../smartcontracts/tests/src/cw20_mutants.rs). For the **candidate LCD wasm**, run [`scripts/layer-a-lcd.sh`](scripts/layer-a-lcd.sh) on LocalTerra (Terra `requires_terra` capability): Transfer / TransferFrom 1:1, allowance backdoor reject, unauthorized mint, idle/snapshot. Send 1:1 to a Receive hook is Layer B (pair Swap + limit).
   - **Layer B (DEX + limits)** — same harness re-runs **P1**, **P2**, **P3**, **P4**/**P10**, **R1–R4**, **C4**, **L1–L3**, **L6**, **L10**, **L11**, sweep on a pair whose one asset is the candidate (other side 10184 analogue). Compare to 10184/10184 control. LocalTerra store of the pinned wasm is `LAYER_B_LT=1` via [`scripts/layer-b-lt.sh`](scripts/layer-b-lt.sh): whitelist **local** store id, `CreatePair`, provide **P2**, round-trip **Send** swap **B7**, limit Send escrow **L1**, SendFrom. Stub PASS is forbidden ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)).
7. **Report** — fill `REPORT.md` from [report-template.md](report-template.md). Include factory-global impact (approving the ID admits every instantiate), instance-admin / migrate residual (**F6**), unverified third-party claim checkbox (**C7**), explicit **go / no-go**.
8. **Optional appendix** — public git URL, CertiK zip, optimizer rebuild hash. **Never** blocks go/no-go. CertiK / Skynet **file** hashes are not `data_hash` (**C7**).

Re-running fetch on an already-fetched ID is idempotent: wasm + `meta.json` + `wasm.sha256` may refresh if LCD still matches; **`REPORT.md` conclusions are never overwritten** by the scripts.

## Parameterization

```bash
CODE_ID=10184 make verify-issue-589   # control (LCD fetch if network)
CODE_ID=8654 make verify-issue-589    # known-bad; 1:1 / P2 must be red
CODE_ID=8266 make verify-issue-589    # first candidate
```

Without `CODE_ID`, `make verify-issue-589` runs fetch/decomp **self-tests**, Layer A/B multi-test (mintable green, FoT mutant red), and docs greps. That is the CI path.

## Control tokens

A check that fails on **10184** (or the in-process mintable analogue) is a **harness bug**, not a listing veto. **8654** / FoT fixtures must fail 1:1 and **P2**; a green 8654 is a harness bug (**C4**).

## Secrets

Do not commit mnemonics, production admin keys, or Coolify tokens under `codeids/` (**C6**). Wasm in git: prefer this download + pin; if a binary must be committed, use Git LFS and still re-hash against LCD in CI.
