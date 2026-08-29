import { useEffect, useMemo, useState } from 'react'
import { resolveAllowedTokenLogoUri } from '@/utils/tokenLogoAllowlist'
import {
  headingWashFromTokenId,
  headingWashBackground,
  sampleLogoWashRgba,
  NEUTRAL_TICKET_HEADER_BACKGROUND,
} from '@/utils/tokenHeadingWash'

/**
 * Ticket header wash keyed by displayed-base token id (#693 A9 / A10).
 * Hash fallback is immediate. Allowlisted https logos may refine via local canvas.
 * No third-party color API.
 */
export function useTokenHeadingWash(tokenId: string, logoURI: string | undefined): string {
  const fallback = useMemo(() => headingWashFromTokenId(tokenId), [tokenId])
  const [sampled, setSampled] = useState<string | null>(null)

  useEffect(() => {
    setSampled(null)
    if (!tokenId) return
    const safe = resolveAllowedTokenLogoUri(logoURI)
    if (!safe) return
    let cancelled = false
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      if (cancelled) return
      const rgba = sampleLogoWashRgba(img)
      if (!cancelled && rgba) setSampled(headingWashBackground(rgba))
    }
    img.onerror = () => {
      /* keep hash fallback */
    }
    img.src = safe
    return () => {
      cancelled = true
      img.onload = null
      img.onerror = null
    }
  }, [tokenId, logoURI])

  if (!tokenId) return NEUTRAL_TICKET_HEADER_BACKGROUND
  return sampled ?? fallback
}
