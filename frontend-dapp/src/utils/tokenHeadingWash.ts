/**
 * Clamped heading wash for `/trade` ticket header (GitLab #693).
 *
 * Color is computed locally from the displayed-base token id (hash) or an
 * allowlisted logo canvas sample. Never interpolates metadata into CSS except
 * numeric `rgba(r, g, b, a)`. Never the leftover orange `rgba(251, 146, 60, …)`.
 */

/** Forbidden leftover warm chrome (#488 / #693 T5). */
export const FORBIDDEN_ORANGE_WASH_RGB = [251, 146, 60] as const

export const WASH_ALPHA = 0.16

/** Neutral header fill when no pair is selected. */
export const NEUTRAL_TICKET_HEADER_BACKGROUND = 'rgba(255, 255, 255, 0.025)'

const ORANGE_HUE_LO = 18
const ORANGE_HUE_HI = 48

function clampByte(n: number): number {
  if (!Number.isFinite(n)) return 0
  return Math.max(0, Math.min(255, Math.round(n)))
}

function relativeLuminance(r: number, g: number, b: number): number {
  const lin = (c: number) => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/** FNV-1a → hue in 0..359, skipping the warm-orange band. */
export function hueFromTokenId(tokenId: string): number {
  let h = 2166136261
  for (let i = 0; i < tokenId.length; i++) {
    h ^= tokenId.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  let hue = (h >>> 0) % 360
  if (hue >= ORANGE_HUE_LO && hue <= ORANGE_HUE_HI) {
    hue = (hue + 55) % 360
  }
  return hue
}

export function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const sat = Math.max(0, Math.min(1, s))
  const light = Math.max(0, Math.min(1, l))
  const c = (1 - Math.abs(2 * light - 1)) * sat
  const hp = (((h % 360) + 360) % 360) / 60
  const x = c * (1 - Math.abs((hp % 2) - 1))
  let r = 0
  let g = 0
  let b = 0
  if (hp < 1) {
    r = c
    g = x
  } else if (hp < 2) {
    r = x
    g = c
  } else if (hp < 3) {
    g = c
    b = x
  } else if (hp < 4) {
    g = x
    b = c
  } else if (hp < 5) {
    r = x
    b = c
  } else {
    r = c
    b = x
  }
  const m = light - c / 2
  return [clampByte((r + m) * 255), clampByte((g + m) * 255), clampByte((b + m) * 255)]
}

/**
 * Pull extreme palettes toward a mid luminance so `--ink` heading stays readable
 * on dark and light (A4). Never returns the forbidden orange triple.
 */
export function clampWashRgb(r: number, g: number, b: number): [number, number, number] {
  let rr = clampByte(r)
  let gg = clampByte(g)
  let bb = clampByte(b)
  const lum = relativeLuminance(rr, gg, bb)
  const MIN_L = 0.22
  const MAX_L = 0.58
  if (lum < MIN_L || lum > MAX_L) {
    const target = lum < MIN_L ? MIN_L : MAX_L
    const scale = lum < 1e-6 ? 1 : target / lum
    rr = clampByte(rr * scale)
    gg = clampByte(gg * scale)
    bb = clampByte(bb * scale)
    if (relativeLuminance(rr, gg, bb) > MAX_L) {
      rr = clampByte(rr * 0.72)
      gg = clampByte(gg * 0.72)
      bb = clampByte(bb * 0.72)
    }
  }
  if (
    rr === FORBIDDEN_ORANGE_WASH_RGB[0] &&
    gg === FORBIDDEN_ORANGE_WASH_RGB[1] &&
    bb === FORBIDDEN_ORANGE_WASH_RGB[2]
  ) {
    rr = 56
    gg = 120
    bb = 196
  }
  return [rr, gg, bb]
}

/** Numeric rgba only — never concatenate untrusted strings. */
export function washRgbaCss(r: number, g: number, b: number, alpha = WASH_ALPHA): string {
  const [rr, gg, bb] = clampWashRgb(r, g, b)
  const a = Number.isFinite(alpha) ? Math.max(0.08, Math.min(0.22, alpha)) : WASH_ALPHA
  return `rgba(${rr}, ${gg}, ${bb}, ${a})`
}

export function headingWashBackground(rgba: string): string {
  return `radial-gradient(circle at 20% 0%, ${rgba}, transparent 34%), ${NEUTRAL_TICKET_HEADER_BACKGROUND}`
}

export function headingWashFromTokenId(tokenId: string): string {
  if (!tokenId) return NEUTRAL_TICKET_HEADER_BACKGROUND
  const [r, g, b] = hslToRgb(hueFromTokenId(tokenId), 0.42, 0.42)
  return headingWashBackground(washRgbaCss(r, g, b))
}

export function cssContainsForbiddenOrangeWash(css: string): boolean {
  return /rgba\(\s*251\s*,\s*146\s*,\s*60\b/i.test(css)
}

/**
 * Average opaque pixels from an already-decoded image. Tainted canvas → null
 * (caller keeps the hash fallback). Does not throw.
 */
export function sampleLogoWashRgba(img: CanvasImageSource): string | null {
  try {
    const canvas = document.createElement('canvas')
    const w = 32
    const h = 32
    canvas.width = w
    canvas.height = h
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx) return null
    ctx.drawImage(img, 0, 0, w, h)
    const data = ctx.getImageData(0, 0, w, h).data
    let r = 0
    let g = 0
    let b = 0
    let n = 0
    for (let i = 0; i < data.length; i += 4) {
      const a = data[i + 3]
      if (a < 24) continue
      const pr = data[i]
      const pg = data[i + 1]
      const pb = data[i + 2]
      const lum = relativeLuminance(pr, pg, pb)
      if (lum > 0.92 || lum < 0.06) continue
      r += pr
      g += pg
      b += pb
      n++
    }
    if (n < 8) return null
    return washRgbaCss(r / n, g / n, b / n)
  } catch {
    return null
  }
}
