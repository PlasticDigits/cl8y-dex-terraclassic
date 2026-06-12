import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const frontendRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..')
const patchesDir = resolve(frontendRoot, 'patches')
const patchHashFile = resolve(patchesDir, '.cosmes-patch-sha256')

function cosmesLockfileVersion(): string {
  const lock = JSON.parse(readFileSync(resolve(frontendRoot, 'package-lock.json'), 'utf8')) as {
    packages?: Record<string, { version?: string }>
  }
  const version = lock.packages?.['node_modules/@goblinhunt/cosmes']?.version
  if (!version) {
    throw new Error('Could not resolve @goblinhunt/cosmes version from package-lock.json')
  }
  return version
}

function findCosmesPatchFile(): string {
  const version = cosmesLockfileVersion()
  const patchPath = resolve(patchesDir, `@goblinhunt+cosmes+${version}.patch`)
  if (!existsSync(patchPath)) {
    throw new Error(
      `No patch file for @goblinhunt/cosmes@${version} (expected patches/@goblinhunt+cosmes+${version}.patch)`
    )
  }
  return patchPath
}

function sha256HexFile(path: string): string {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

function readCosmes(relPath: string): string {
  return readFileSync(resolve(frontendRoot, 'node_modules/@goblinhunt/cosmes', relPath), 'utf8')
}

describe('cosmes patch-package (GitLab #127, #367)', () => {
  it('patch file SHA-256 matches committed patches/.cosmes-patch-sha256', () => {
    const patchPath = findCosmesPatchFile()
    const actual = sha256HexFile(patchPath)
    const expected = readFileSync(patchHashFile, 'utf8').trim().split(/\s+/)[0]
    expect(actual).toBe(expected)
  })

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
