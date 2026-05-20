import { describe, it, expect } from 'vitest'
import {
  MEV_POSTURE_DOCS_ANCHOR,
  MEV_POSTURE_DOCS_PATH,
  MEV_POSTURE_HEADING,
  MEV_POSTURE_SUMMARY,
  MEV_POSTURE_SLIPPAGE_NOTE,
  MEV_POSTURE_NO_OPT_IN,
} from './mevPosture'

describe('mevPosture copy', () => {
  it('anchors docs for cross-links', () => {
    expect(MEV_POSTURE_DOCS_PATH).toContain(MEV_POSTURE_DOCS_ANCHOR)
    expect(MEV_POSTURE_DOCS_PATH).toMatch(/frontend\.md/)
  })

  it('states public mempool and no MEV relay', () => {
    expect(MEV_POSTURE_HEADING).toMatch(/MEV/i)
    expect(MEV_POSTURE_SUMMARY).toMatch(/public/i)
    expect(MEV_POSTURE_SUMMARY).toMatch(/does not offer/i)
    expect(MEV_POSTURE_NO_OPT_IN).toMatch(/no opt-in/i)
  })

  it('ties protection to slippage', () => {
    expect(MEV_POSTURE_SLIPPAGE_NOTE).toMatch(/slippage/i)
    expect(MEV_POSTURE_SLIPPAGE_NOTE).toMatch(/max spread/i)
  })
})
