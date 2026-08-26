import { getNativeEquivalent } from '@/types'

/**
 * Advanced two-sided provide: auto-wrap checkbox defaults **on** iff this pair
 * leg has a native equivalent (cLUNC→uluna, cUSTC→uusd). Session-only; not persisted.
 * GitLab #661 **P661-5**.
 */
export function provideWrapDefaultOn(pairLegTokenId: string): boolean {
  if (!pairLegTokenId) return false
  return getNativeEquivalent(pairLegTokenId) != null
}
