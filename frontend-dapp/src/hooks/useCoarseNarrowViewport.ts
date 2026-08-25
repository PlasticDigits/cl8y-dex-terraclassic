import { useEffect, useState } from 'react'
import { isCoarseNarrowViewport } from '@/lib/coarseNarrowViewport'

/**
 * Live coarse+narrow detect for browse-without-IME pickers (GitLab #632).
 * Initial state matches the current window so the first paint does not flash
 * a text field on phones.
 */
export function useCoarseNarrowViewport(): boolean {
  const [coarseNarrow, setCoarseNarrow] = useState(() =>
    typeof window === 'undefined' ? false : isCoarseNarrowViewport()
  )

  useEffect(() => {
    const sync = () => setCoarseNarrow(isCoarseNarrowViewport())
    sync()
    const mql = window.matchMedia('(pointer: coarse)')
    mql.addEventListener('change', sync)
    window.addEventListener('resize', sync)
    return () => {
      mql.removeEventListener('change', sync)
      window.removeEventListener('resize', sync)
    }
  }, [])

  return coarseNarrow
}
