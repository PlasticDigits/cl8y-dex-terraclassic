# How to add LUNC liquidity

Plain-language steps for **dex.cl8y.com**. This is **not** financial advice.

The in-app guide lives on **Pool** (`/pool#lp-howto`). You do not need this GitLab page to add liquidity.

Engineering invariants: [`frontend.md` § Retail LUNC liquidity how-to](./frontend.md#retail-lunc-liquidity-howto), [§ One-sided pool add / withdraw](./frontend.md#pool-one-sided-liquidity), and [§ Pool Manage IA](./frontend.md#pool-manage-ia). Incident pause/blacklist: [`user-incident-faq.md`](./user-incident-faq.md).

## Two different actions

People say “provide LUNC liquidity” for two products:

1. **v2 pool LP** — on **Pool**, expand **Manage** on a pair. **Provide Liquidity** deposits both tokens. **Zap Add** uses **one token** you already hold (the other side is swapped to the pool ratio). You receive **LP tokens** (your share). **Withdraw Liquidity** returns both assets; **Zap Withdraw** exits as one token. Empty pools need Provide Liquidity.
2. **Maker limits** — escrow one token at a price on **Trade** or **Limits**. That is **not** a pool share.

There is **no** LP or maker incentive program currently. No APR or points.

## Before you sign

Always verify the **network badge** and the environment strip below the header. Sign only on this site.

Keep **bank LUNC** for gas. Pools hold **wrapped** LUNC (cLUNC), not bank LUNC.

## Add LP (Pool)

1. Connect your wallet.
2. Open **Pool** (header, **More** on a tablet, or the phone tab).
3. Expand **Manage** on the pair. Choose **Provide Liquidity**, **Withdraw Liquidity**, **Zap Add**, or **Zap Withdraw**. Native LUNC auto-wraps on Zap Add — no checkbox.
4. Read the impermanent-loss notice and the pre-sign summary, then confirm.
5. Keep the **LP tokens**. They are your pool share.

Empty pools cannot use Zap Add. Use **Provide Liquidity** for the first deposit.

Zap Add swaps to the pool ratio so extra is not donated. Provide Liquidity can still donate if amounts are off-ratio.

## Withdraw

Pool → Manage → **Withdraw Liquidity** (both assets) or **Zap Withdraw** (one token) → amount → review the preview → sign.

Unwrapping is **not** free LUNC out. Use **Wrap** for fee quotes. Do not send unwrapped LUNC to an exchange without the wrap-page warning.

## Optional: maker limits

Trade or Limits → Buy/Sell → price → Pay → Place limit. Escrow is **not** LP and **not** a rewards program.

## Create Pair fees

**Create Pair** (under More) is only for a **new** market. That page charges a **LUNC creation fee** plus gas. You do not need it to add liquidity to an existing pool.

## What this page is not

- Not a farm, airdrop, or “send LUNC to this address” guide.
- Not a replacement for pause, blacklist, gas, ratio, clickwrap, or risk gates on the dApp.
- Not the incident FAQ.
