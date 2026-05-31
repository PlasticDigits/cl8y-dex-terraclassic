/** Milliseconds before the typeahead buffer clears (WAI-ARIA listbox APG). */
export const PORTAL_LISTBOX_TYPEAHEAD_RESET_MS = 500

export function wrapPortalListboxIndex(index: number, length: number): number {
  if (length <= 0) return 0
  return ((index % length) + length) % length
}

export function movePortalListboxActiveIndex(current: number, delta: number, length: number): number {
  return wrapPortalListboxIndex(current + delta, length)
}

/**
 * First option whose label starts with `query` (case-insensitive), searching forward from
 * `startFrom` and wrapping — WAI-ARIA listbox typeahead.
 */
export function findPortalListboxTypeaheadIndex(
  labels: readonly string[],
  query: string,
  startFrom: number
): number | null {
  if (!query || labels.length === 0) return null
  const q = query.toLowerCase()
  for (let step = 1; step <= labels.length; step++) {
    const idx = wrapPortalListboxIndex(startFrom + step, labels.length)
    if (labels[idx].toLowerCase().startsWith(q)) return idx
  }
  return null
}

export function portalListboxOptionId(listId: string, index: number): string {
  return `${listId}-option-${index}`
}
