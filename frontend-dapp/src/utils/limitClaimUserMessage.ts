/**
 * Retail copy for `ClaimExpiredLimitOrder` failures (GitLab #141).
 * Wired via {@link tryHumanizeTerraTxMessage} / {@link handleTransactionError}.
 */

export function humanizeExpiredLimitClaimMessage(raw: string): string | null {
  const inner = raw.replace(/\s+/g, ' ').trim()
  if (/No claimable expired-limit refund for order id/i.test(inner)) {
    return (
      'Nothing to claim for this order id — it may already be refunded, still active on the book, ' +
      'or the indexer may not have caught up yet. Refresh in a few blocks or check the explorer.'
    )
  }
  return null
}
