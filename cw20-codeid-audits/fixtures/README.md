# Control fixtures (GitLab #589)

No live keys. No mnemonics.

| Fixture | Role | Expected |
|---------|------|----------|
| In-process `cw20-mintable` | **10184** analogue (protocol mintable) | Honest Layer A/B **green** |
| LCD **6036** | Listed TerraSwap token | Green or documented exceptions in `codeids/6036/REPORT.md` |
| LCD **8266** | Terraport token V2 / SpaceUSD template | See `codeids/8266/REPORT.md` — listing still #581 |
| LCD **8654** | ALPHA `cw20-taxed` (`tax_map`) | 1:1 and **P2** **FAIL** |
| [`cw20_mutants.rs`](../../smartcontracts/tests/src/cw20_mutants.rs) | Generated malice (G2) | Each detector row **must fire** |

ALPHA 8654 is a **known-bad** control. Marking it GO is a reviewer veto (**C4**).
