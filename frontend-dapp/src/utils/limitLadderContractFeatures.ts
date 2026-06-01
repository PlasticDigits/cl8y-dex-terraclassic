/**
 * Contract capability gates for ladder placement (GitLab #266 / #268).
 *
 * Book-order batch traversal + `LimitOrderLadderSpec.hint_after_order_id` shipped in #266.
 * Set `VITE_LIMIT_LADDER_SINGLE_ANCHOR=false` only when targeting legacy pair wasm
 * without the boundary anchor field.
 */
export function supportsLadderSingleAnchor(): boolean {
  return import.meta.env.VITE_LIMIT_LADDER_SINGLE_ANCHOR !== 'false'
}
