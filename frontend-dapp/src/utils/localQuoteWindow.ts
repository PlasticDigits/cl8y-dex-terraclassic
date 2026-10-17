/**
 * Reviewed local quote signing window (accessibility policy: 30 minutes).
 *
 * Direct/native local quotes stay signable for this long so that disabled users
 * and assistive-technology users are not forced to race a wallet popup. The
 * on-chain `minimum_receive` / `max_spread` floors remain the economic
 * protection if prices or fees move while the reviewed quote is open.
 *
 * Must stay in sync with the indexer's `LOCAL_QUOTE_SIGNING_WINDOW_MS`
 * (`indexer/src/api/local_quote.rs`).
 */
export const LOCAL_QUOTE_SIGNING_WINDOW_MS = 30 * 60 * 1000
