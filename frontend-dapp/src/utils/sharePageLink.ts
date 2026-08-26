import { isValidTerraAddress } from '@/utils/constants'
import { copyToClipboard, type CopyToClipboardResult } from '@/utils/copyToClipboard'
import { SHARE_LINK_TITLE } from '@/utils/sharePageLinkCopy'
import { shortenAddress } from '@/utils/tokenDisplay'

export const SHARE_PAGE_KINDS = ['trader', 'trade', 'charts'] as const
export type SharePageKind = (typeof SHARE_PAGE_KINDS)[number]

export type SharePayload = {
  url: string
  title: string
  text: string
}

export type ShareFn = (data: SharePayload) => Promise<void>
export type CanShareFn = (data: SharePayload) => boolean
export type CopyFn = (text: string) => Promise<CopyToClipboardResult>

export type ShareOrCopyResult =
  | { outcome: 'shared' }
  | { outcome: 'aborted' }
  | { outcome: 'copied' }
  | { outcome: 'copy-failed'; message: string }

const KIND_SET = new Set<string>(SHARE_PAGE_KINDS)

/**
 * Canonical same-origin share URL: `{origin}/{kind}/{validatedId}` with no search or hash.
 * Rejects invalid terra addresses, non-http(s) origins, and unknown kinds (TS-2 / TS-9).
 * Never treats a raw user string as `href` — origin is parsed, path is built from an allowlist.
 */
export function buildCanonicalShareUrl(input: { origin: string; kind: SharePageKind; id: string }): string | null {
  if (!KIND_SET.has(input.kind)) return null

  const id = input.id.trim()
  if (!isValidTerraAddress(id)) return null
  if (/[/?#\\]/.test(id)) return null

  const originRaw = input.origin.trim()
  if (!originRaw) return null

  let parsed: URL
  try {
    parsed = new URL(originRaw)
  } catch {
    return null
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null
  if (parsed.username || parsed.password) return null

  const out = new URL(parsed.origin)
  out.pathname = `/${input.kind}/${id}`
  out.search = ''
  out.hash = ''
  return out.href
}

export function traderShareText(address: string): string {
  return `${SHARE_LINK_TITLE} ${shortenAddress(address.trim())}`
}

export function isShareAbortError(err: unknown): boolean {
  if (!err || typeof err !== 'object') return false
  const name = 'name' in err ? String((err as { name?: unknown }).name) : ''
  return name === 'AbortError'
}

export function resolveNavigatorShare(): ShareFn | undefined {
  if (typeof navigator === 'undefined' || typeof navigator.share !== 'function') return undefined
  return (data) => navigator.share(data)
}

export function resolveNavigatorCanShare(): CanShareFn | undefined {
  if (typeof navigator === 'undefined' || typeof navigator.canShare !== 'function') return undefined
  return (data) => {
    try {
      return navigator.canShare(data)
    } catch {
      return false
    }
  }
}

/**
 * Prefer Web Share on a user gesture; AbortError is not a failure.
 * Missing share, canShare=false, or non-abort errors fall back to `copyToClipboard`.
 */
export async function shareOrCopyPageLink(
  payload: SharePayload,
  deps: {
    share?: ShareFn
    canShare?: CanShareFn
    copy?: CopyFn
  } = {}
): Promise<ShareOrCopyResult> {
  const share = deps.share
  const canShare = deps.canShare
  const shareAllowed = Boolean(share) && (canShare == null || canShare(payload) !== false)

  if (share && shareAllowed) {
    try {
      await share(payload)
      return { outcome: 'shared' }
    } catch (err) {
      if (isShareAbortError(err)) return { outcome: 'aborted' }
    }
  }

  const copy = deps.copy ?? copyToClipboard
  const result = await copy(payload.url)
  if (result.ok) return { outcome: 'copied' }
  return { outcome: 'copy-failed', message: result.message }
}
