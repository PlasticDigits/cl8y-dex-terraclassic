import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const frontendRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..')

function readCosmes(relPath: string): string {
  return readFileSync(resolve(frontendRoot, 'node_modules/@goblinhunt/cosmes', relPath), 'utf8')
}

describe('cosmes patch-package (GitLab #127)', () => {
  it('KeplrExtension passes per-sign preferNoSetFee and post-sign fee guard', () => {
    const src = readCosmes('dist/wallet/wallets/keplr/KeplrExtension.js')
    expect(src).toContain('EXTENSION_SIGN_OPTIONS')
    expect(src).toContain('preferNoSetFee: true')
    expect(src).toContain('assertExtensionSignedFeeMeetsExpected')
    expect(src).toContain('assertExtensionSignedDirectFeeMeetsExpected')
    expect(src).toContain('EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX')
    expect(src).toContain('meetsMinSignedRatio')
    expect(src).toContain('ulunaFromAminoFee')
    expect(src).toContain('ulunaFromDirectSignedAuthInfoBytes')
    expect(src).toContain('toPlainAminoStdSignDoc')
    expect(src).toContain('stdDoc.fee')
    expect(src).not.toContain('for (let attempt = 0; attempt < 2; attempt++)')
    expect(src).toMatch(/GitLab #208|extension popup was closed/)
  })

  it('StationController always uses amino signing for extension (GitLab #208)', () => {
    const src = readCosmes('dist/wallet/wallets/station/StationController.js')
    expect(src).toContain('useAminoSigning = true')
    expect(src).toMatch(/GitLab #127|#208/)
  })
})
