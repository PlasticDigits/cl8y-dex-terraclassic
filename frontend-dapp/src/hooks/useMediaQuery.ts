import { useEffect, useState } from 'react'

function queryMatches(query: string): boolean {
  if (typeof window === 'undefined') return false
  return window.matchMedia(query).matches
}

/**
 * Subscribes to `window.matchMedia(query)`. Initial state matches the live query on the client
 * to reduce layout flicker on first paint.
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => queryMatches(query))

  useEffect(() => {
    const mql = window.matchMedia(query)
    const sync = () => setMatches(mql.matches)
    sync()
    mql.addEventListener('change', sync)
    return () => mql.removeEventListener('change', sync)
  }, [query])

  return matches
}
