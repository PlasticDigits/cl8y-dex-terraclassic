import { describe, expect, it } from 'vitest'

import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useTokenBalance } from '@/hooks/useTokenBalance'

describe('useTokenBalance (GitLab #231)', () => {
  it('re-exports useLimitOrderEscrowBalance so ladder gates and limit place share one hook', () => {
    expect(useTokenBalance).toBe(useLimitOrderEscrowBalance)
  })
})
