/** @vitest-environment node */
import { readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { OG_IMAGE_HEIGHT, OG_IMAGE_WIDTH } from '../../../viteOg'

const frontendRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..')
const ogPath = path.join(frontendRoot, 'public/og-image.png')
const conceptPath = path.join(frontendRoot, 'brand/community-opengraph-concept.png')

function pngIhdr(buf: Buffer): { width: number; height: number } {
  expect(buf.subarray(0, 8).toString('hex')).toBe('89504e470d0a1a0a')
  expect(buf.toString('ascii', 12, 16)).toBe('IHDR')
  return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) }
}

describe('og-image.png binary (GitLab #578)', () => {
  it('is a 1200x630 PNG under 5 MB', () => {
    const buf = readFileSync(ogPath)
    const { width, height } = pngIhdr(buf)
    expect(width).toBe(OG_IMAGE_WIDTH)
    expect(height).toBe(OG_IMAGE_HEIGHT)
    expect(statSync(ogPath).size).toBeLessThan(5_000_000)
    expect(statSync(ogPath).size).toBeGreaterThan(100_000)
  })

  it('keeps the square community concept as source (not the crawler URL)', () => {
    const concept = readFileSync(conceptPath)
    const { width, height } = pngIhdr(concept)
    expect(width).toBe(1254)
    expect(height).toBe(1254)
    expect(statSync(conceptPath).size).toBeGreaterThan(statSync(ogPath).size)
  })
})
