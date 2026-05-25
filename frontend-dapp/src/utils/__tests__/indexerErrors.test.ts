import { describe, expect, it } from 'vitest'
import { isIndexerPairNotFoundError, isIndexerUnavailableError } from '@/utils/indexerErrors'

describe('indexerErrors', () => {
  it('treats indexer 404 as pair-not-found, not outage', () => {
    const err = new Error('Indexer API error: 404 Not Found')
    expect(isIndexerPairNotFoundError(err)).toBe(true)
    expect(isIndexerUnavailableError(err)).toBe(false)
  })

  it('treats indexer 502 as outage', () => {
    const err = new Error('Indexer API error: 502 Bad Gateway')
    expect(isIndexerPairNotFoundError(err)).toBe(false)
    expect(isIndexerUnavailableError(err)).toBe(true)
  })
})
