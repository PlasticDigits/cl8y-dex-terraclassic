# #642 / cosmos chain-registry #7910 review follow-up

Upstream PR: <https://github.com/cosmos/chain-registry/pull/7910>.
Pack: [cosmos-chain-registry](../../listings/cosmos-chain-registry/README.md).
Snapshot: [evidence-2026-10-04.json](./evidence-2026-10-04.json).

On 2026-10-04, public columbus-5 LCD queries confirmed code ID **10184** and
decimals for all seven CW20s. The code hash is
`C77FFCDB0039041FBD071C76EFB547232314CE8CCF219C3BF5B7FABD4CF7D4A4`.
The new USDT entry is exactly the #1260 contract, name **Tether USD**, symbol
**USDT**, decimals **18**. The bridge maps CL8Y, vFDUSD, and USDT to their BSC
origins; vFDUSD's destination scale is 8 while Terra uses 6.

The review's minter question is resolved by querying both CW20 endpoints.
cLUNC/cUSTC retain a primary mint administrator and list the wrap mapper
`terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2`
as an additional minter. The mapper's config identifies the CMM treasury.

The CL8Y backing question must distinguish launch issuance from bridge
issuance. Contract history records an initial **500,000 CL8Y** balance at
Terra block **24,103,161**, followed by migration from code 10181 to 10184.
This allocation was created in the instantiate message, not in a subsequent
bridge withdrawal. Terra's current CL8Y bridge mode is **MintBurn**; BSC's is
**LockUnlock**. Terra's observed supply is **531,346.822618733285100506 CL8Y**.
BSC LockUnlock holds **150,904.259569328070235803 CL8Y** at block **125,587,275**.
MegaETH also has a CL8Y representation, observed at
**122,746.867338082963797604 CL8Y**. These independent observations do not
establish an exact multi-chain reserve reconciliation. Neither the full Terra
supply nor the whole shared BSC escrow is a Terra-only bridge liability.
The descriptions disclose the launch allocation and do not promise that all
Terra supply is locked on BSC. Older custody addresses checked hold zero.

USDT's observed Terra supply is **11,710.648813487453250379** and BSC escrow is
**1,710.648813487453250379**. The issuer's `market` query records outstanding
debt of exactly **10,000 USDT**, explaining that difference. This is local debt
issuance, not an additional BSC reserve. The registry description discloses it.

Validation completed locally in the upstream checkout:

- `@chain-registry/cli@1.47.0` schema validation: **2,282 checks passed**.
- ESLint on both changed assetlists: passed.
- Upstream `validate_data.mjs` with live CoinGecko fetching: passed.
- Six submitted PNGs: genuine PNGs, 256×256, each below 250 kB.
- `make verify-issue-642`: pins, origin references/scales, mint roles, launch
  allocation, image files, and debt disclosure checked against the snapshot.

Upstream merge and Leap/Directory propagation remain pending. #1260's
chain-registry portion is submitted here; this record does not claim its other
wallet venues or shared-catalog work are complete.
