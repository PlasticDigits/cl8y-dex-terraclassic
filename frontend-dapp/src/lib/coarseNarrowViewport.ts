/** Phone-width breakpoint used with coarse pointer (GitLab #632 browse-without-IME). */
export const NARROW_VIEWPORT_MAX_PX = 767

export type CoarseNarrowEnv = {
  innerWidth: number
  matchMedia?: (query: string) => Pick<MediaQueryList, 'matches'>
}

/**
 * Coarse pointer **and** width ≤767. Desktop DevTools (fine pointer, narrow width)
 * keeps type-to-filter; real phones / in-app WebViews take the browse path.
 */
export function isCoarseNarrowViewport(
  env: CoarseNarrowEnv | Window = typeof window !== 'undefined' ? window : { innerWidth: 1024 }
): boolean {
  const width = typeof env.innerWidth === 'number' && Number.isFinite(env.innerWidth) ? env.innerWidth : 1024
  const matchMedia = env.matchMedia?.bind(env)
  const coarse = matchMedia ? matchMedia('(pointer: coarse)').matches : false
  return coarse && width <= NARROW_VIEWPORT_MAX_PX
}
