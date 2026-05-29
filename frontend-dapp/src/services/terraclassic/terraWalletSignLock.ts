/**
 * Serializes extension wallet sign/broadcast calls so only one Station/Keplr popup is active.
 * Overlapping prompts cause false "User denied, extension popup was closed" (GitLab #208).
 */

let signChain: Promise<unknown> = Promise.resolve()

export function withTerraWalletSignLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = signChain.then(fn, fn)
  signChain = run.then(
    () => undefined,
    () => undefined
  )
  return run
}

/** @internal Reset between unit tests. */
export function resetTerraWalletSignLockForTests(): void {
  signChain = Promise.resolve()
}
