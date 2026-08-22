# REPORT: CW20 code ID \<ID\>

**Date:**  
**Operator:**  
**LCD:**  
**Procedure:** [`PROCEDURE.md`](PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))

The inspected binary is a **decompilation / fingerprint of LCD wasm**, not redistributable source. F6 pin does not replace this report.

## Verdict

**GO / NO-GO** for factory `AddWhitelistedCodeId <ID>`:

- [ ] GO — Layer A + Layer B green on the **pinned LCD wasm**; catalogue rows pass or N/A+reason; residuals written
- [ ] NO-GO — fail closed (do not whitelist; do not add FoT math)

One-line reason:

## Identity

| Field | Value |
|-------|--------|
| `code_id` | |
| `data_hash` (LCD) | |
| SHA-256 of downloaded wasm | |
| Match | **yes / FAIL** |
| Creator / uploader | |
| Instantiate permission | Everybody / Nobody / … |
| Approximate instantiate count | |
| `meta.json` | `codeids/<id>/meta.json` |

## Fetch

- Endpoints used (G4):
- Hash mismatch / truncated download: **did not occur / FAIL closed**

## Fingerprint

Exports, rustc / optimizer line, crate strings, custom enums, host imports (`requires_terra`, `ibc_*`):

## Decompile

Paths under `codeids/<id>/decomp/`. Unreadable regions:

## Catalogue

Copy pass / fail / N/A+reason for **every** row in [`CATALOG.md`](CATALOG.md) (A1–A30, B1–B15, C1–C7, D1–D22, E1–E15, catalog F1–F8, G1–G9, CH1–CH18). Do not leave a row blank.

| ID | Result | Notes |
|----|--------|-------|
| A1 | | |
| … | | |

## Layer A (token-only)

Backend (multi-test mintable / LocalTerra LCD wasm / live snapshot):

Log / JUnit pointer:

## Layer B (DEX + limits)

Backend (multi-test / LocalTerra). If LocalTerra was unavailable, write the explicit skip string (not a silent pass):

Invariants re-run: P1, P2, P3, P4/P10, R1–R4, C4, L1–L3, L6, L10, L11, sweep.

## Factory-global impact

Approving this ID admits **every current and future instantiate** of this wasm (not only the token that motivated the request).

## Instance admin / migrate residual (F6)

Wasm admin, minter, marketing admin. `MsgMigrateContract` still freeze-trades until Refresh. Residual:

## Unverified third-party claim (C7)

- [ ] I did **not** treat CertiK / Skynet **file** hashes or marketing copy as LCD `data_hash`
- [ ] Optional appendix (source URL / rebuild) does **not** block this verdict

## Appendix (optional — never a gate)

Source URL, CertiK zip, optimizer rebuild SHA, live-instance sample (G5):
