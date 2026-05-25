import { describe, expect, it, vi } from 'vitest'
import { COPY_BUTTON_FAILURE_MESSAGE } from '@/utils/copyButtonCopy'
import { copyToClipboard } from '@/utils/copyToClipboard'

describe('copyToClipboard', () => {
  it('writes trimmed text on success', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    const result = await copyToClipboard('  terra1abc  ', { writeText })
    expect(result).toEqual({ ok: true })
    expect(writeText).toHaveBeenCalledWith('terra1abc')
  })

  it('fails for whitespace-only text without calling writeText', async () => {
    const writeText = vi.fn()
    const result = await copyToClipboard('   ', { writeText })
    expect(result).toEqual({ ok: false, message: COPY_BUTTON_FAILURE_MESSAGE })
    expect(writeText).not.toHaveBeenCalled()
  })

  it('returns retail-safe message when writeText rejects', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('denied'))
    const result = await copyToClipboard('terra1xyz', { writeText })
    expect(result).toEqual({ ok: false, message: COPY_BUTTON_FAILURE_MESSAGE })
  })
})
