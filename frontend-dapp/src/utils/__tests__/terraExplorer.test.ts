import { describe, it, expect } from 'vitest'
import { shortenTxHashForDisplay } from '../terraExplorer'

describe('shortenTxHashForDisplay', () => {
  it('middle-elides long hashes', () => {
    const h = `AAAAAAAA${'0'.repeat(50)}BBBBBB`
    expect(shortenTxHashForDisplay(h)).toBe('AAAAAAAA…BBBBBB')
  })

  it('returns short strings unchanged', () => {
    expect(shortenTxHashForDisplay('abc')).toBe('abc')
  })
})
