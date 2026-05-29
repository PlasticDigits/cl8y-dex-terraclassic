import { describe, it, expect, vi } from 'vitest'
import { resetTerraWalletSignLockForTests, withTerraWalletSignLock } from '../terraWalletSignLock'

describe('withTerraWalletSignLock (GitLab #208)', () => {
  it('runs broadcasts one at a time', async () => {
    resetTerraWalletSignLockForTests()
    const order: number[] = []
    const first = withTerraWalletSignLock(async () => {
      order.push(1)
      await new Promise((r) => setTimeout(r, 30))
      order.push(2)
      return 'a'
    })
    const second = withTerraWalletSignLock(async () => {
      order.push(3)
      return 'b'
    })
    expect(await Promise.all([first, second])).toEqual(['a', 'b'])
    expect(order).toEqual([1, 2, 3])
  })

  it('releases the lock after failure', async () => {
    resetTerraWalletSignLockForTests()
    const fn = vi.fn().mockResolvedValue('ok')
    await expect(
      withTerraWalletSignLock(async () => {
        throw new Error('fail')
      })
    ).rejects.toThrow('fail')
    await withTerraWalletSignLock(fn)
    expect(fn).toHaveBeenCalledTimes(1)
  })
})
