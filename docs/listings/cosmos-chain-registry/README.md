# Cosmos chain-registry / Leap / Directory

Upstream: [cosmos/chain-registry#7910](https://github.com/cosmos/chain-registry/pull/7910).
DEX tracking: [#642](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/642).

The Terra Classic fragment contains the permanent six CW20s plus the single
USDT exception requested by [#1260](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1260).
That issue's Keplr, Hexxagon, and shared-catalog deliverables remain separate.
The parent #639 catalog remains locked to its permanent six.

`terra/assetlist.fragment.json` belongs in `terra/assetlist.json` (columbus-5).
The BSC fragment contains new CL8Y and Venus vFDUSD origin entries and the
existing USDT entry with its exponent corrected to the live ERC20's 18 decimals.
Append the new origins and amend the existing USDT entry; do not duplicate it.

CL8Y and vFDUSD use `bridge` traces to registered BSC origins. CL8Y's
`ceramicliberty-com` CoinGecko ID belongs only on its BSC entry. vFDUSD is a Venus
receipt token with 8 decimals on BSC and 6 on Terra. The mapper's recorded
`dest_decimals` confirms the conversion. It is not native FDUSD.

cLUNC and cUSTC use same-chain `wrapped` traces to `uluna` and `uusd`. The
wrap mapper is an additional authorized minter; each token also has a primary
mint administrator. Descriptions disclose both roles. UST1 remains an
unstablecoin; USTR remains USTC Repeg and is not a stablecoin.

CL8Y's original Terra instantiate message records a **500,000-token launch
allocation** at block **24,103,161**. Terra's bridge mode is MintBurn; BSC's
is LockUnlock. The full Terra supply is therefore not a measure of tokens
minted against the current BSC LockUnlock balance. See the dated
[chain evidence](../../qa/issue-642/evidence-2026-10-04.json) and
[QA record](../../qa/issue-642/README.md).

USDT's bridge origin is the existing BSC USDT entry. The Terra supply also
includes **10,000 USDT** issued locally through Bridge-Debt, confirmed by the
issuer's `market` query. Its description discloses this local issuance; it
does not claim that all supply is locked on BSC or add a CoinGecko ID.

Images follow upstream's origin-only policy: CL8Y/vFDUSD PNGs live under BSC;
the Terra assets use `image_sync`. USDT reuses upstream's existing source
images. `source-images/USDT.png` preserves the exact DEX tokenlist logo for
#1260's offline pack; it is not a new upstream Terra logo.

Run `make verify-issue-642`. The verifier checks pinned contracts/decimals,
recorded bridge mappings and origin scales, minter roles, traces, source
images, debt disclosure, and excluded-token boundaries. It is an offline
regression check against a dated snapshot, not a fresh reserve attestation.
Upstream schema and data validation run in the chain-registry checkout.
