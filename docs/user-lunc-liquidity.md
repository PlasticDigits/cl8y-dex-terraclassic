# How to add LUNC liquidity

Plain-language steps for **dex.cl8y.com**. This is **not** financial advice.

The in-app guide lives on **Pool** (`/pool#lp-howto`). You do not need this GitLab page to add liquidity.

Engineering invariants: [`frontend.md` § Retail LUNC liquidity how-to](./frontend.md#retail-lunc-liquidity-howto) and [§ One-sided pool add / withdraw](./frontend.md#pool-one-sided-liquidity). Incident pause/blacklist: [`user-incident-faq.md`](./user-incident-faq.md).

## Two different actions

People say “provide LUNC liquidity” for two products:

1. **v2 pool LP** — add **one token** you already hold on **Pool** (the other side is swapped to the pool ratio). You receive **LP tokens** (your share). Withdraw on Pool as a single token. Two-sided deposit is **Advanced** (empty pools).
2. **Maker limits** — escrow one token at a price on **Trade** or **Limits**. That is **not** a pool share.

There is **no** LP or maker incentive program currently. No APR or points.

## Before you sign

Always verify the **network badge** and the environment strip below the header. Sign only on this site.

Keep **bank LUNC** for gas. Pools hold **wrapped** LUNC (cLUNC), not bank LUNC.

## Add LP (Pool)

1. Connect your wallet.
2. Open **Pool** (header, **More** on a tablet, or the phone tab).
3. Pick **Token** (a holding), **Pair**, and **Amount**. Native LUNC auto-wraps — no checkbox.
4. Read the impermanent-loss notice and the pre-sign summary, then **Add**.
5. Keep the **LP tokens**. They are your pool share.

Empty pools cannot use one-sided Add. Use **Advanced** two-sided for the first deposit.

Retail Add swaps to the pool ratio so extra is not donated. **Advanced** two-sided can still donate if amounts are off-ratio.

## Withdraw

Pool → pick **LP** → **Withdraw as** one token → amount → review the preview → sign.

Unwrapping is **not** free LUNC out. Use **Wrap** for fee quotes. Do not send unwrapped LUNC to an exchange without the wrap-page warning.

## Optional: maker limits

Trade or Limits → Buy/Sell → price → Pay → Place limit. Escrow is **not** LP and **not** a rewards program.

## Create Pair fees

**Create Pair** (under More) is only for a **new** market. That page charges a **LUNC creation fee** plus gas. You do not need it to add liquidity to an existing pool.

## What this page is not

- Not a farm, airdrop, or “send LUNC to this address” guide.
- Not a replacement for pause, blacklist, gas, ratio, clickwrap, or risk gates on the dApp.
- Not the incident FAQ.
