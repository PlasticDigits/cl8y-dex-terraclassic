# How to add LUNC liquidity

Plain-language steps for **dex.cl8y.com**. This is **not** financial advice.

The in-app guide lives on **Pool** (`/pool#lp-howto`). You do not need this GitLab page to add liquidity.

Engineering invariants: [`frontend.md` § Retail LUNC liquidity how-to](./frontend.md#retail-lunc-liquidity-howto). Incident pause/blacklist: [`user-incident-faq.md`](./user-incident-faq.md).

## Two different actions

People say “provide LUNC liquidity” for two products:

1. **v2 pool LP** — add **both** tokens of a pair on **Pool**. You receive **LP tokens** (your share). Withdraw on Pool.
2. **Maker limits** — escrow one token at a price on **Trade** or **Limits**. That is **not** a pool share.

There is **no** LP or maker incentive program currently. No APR or points.

## Before you sign

Always verify the **network badge** and the environment strip below the header. Sign only on this site.

Keep **bank LUNC** for gas. Pools hold **wrapped** LUNC (cLUNC), not bank LUNC.

## Add LP (Pool)

1. Connect your wallet.
2. Open **Pool** (header, **More** on a tablet, or the phone tab).
3. Pick a LUNC or cLUNC pair.
4. Choose **Provide**. Enter **both** amounts (the other side auto-fills). A LUNC-only deposit is not a v2 pool action.
5. Optional: check **Use native LUNC (auto-wrap)**, or wrap first under More → **Wrap**.
6. Read the impermanent-loss notice and the pre-sign summary, then sign. Two wrapped tokens can take **three** wallet prompts — do not skip the gas check.
7. Keep the **LP tokens**. They are your pool share.

If amounts are off the pool ratio, the extra is **donated** to the pool.

## Withdraw

Pool → **Withdraw** → enter LP amount → review the preview → sign.

Unwrapping is **not** free LUNC out. Use **Wrap** for fee quotes. Do not send unwrapped LUNC to an exchange without the wrap-page warning.

## Optional: maker limits

Trade or Limits → Buy/Sell → price → Pay → Place limit. Escrow is **not** LP and **not** a rewards program.

## Create Pair fees

**Create Pair** (under More) is only for a **new** market. That page charges a **LUNC creation fee** plus gas. You do not need it to add liquidity to an existing pool.

## What this page is not

- Not a farm, airdrop, or “send LUNC to this address” guide.
- Not a replacement for pause, blacklist, gas, ratio, clickwrap, or risk gates on the dApp.
- Not the incident FAQ.
