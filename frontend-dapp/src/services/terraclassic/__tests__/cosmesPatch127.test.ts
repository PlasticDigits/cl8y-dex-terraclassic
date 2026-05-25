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
    expect(src).toContain('EXTENSION_SIGNED_FEE_MIN_PERCENT')
    expect(src).toContain('gasFromAminoFee')
    expect(src).toContain('gasFromDirectSignedAuthInfoBytes')
    expect(src).toContain('for (let attempt = 0; attempt < 2; attempt++)')
  })

  it('StationController uses amino signing for LocalTerra (case-insensitive)', () => {
    const src = readCosmes('dist/wallet/wallets/station/StationController.js')
    expect(src).toContain('isLocalTerraChain')
    expect(src).toContain('toLowerCase() === "localterra"')
    expect(src).toContain('useAminoSigning')
  })
})
