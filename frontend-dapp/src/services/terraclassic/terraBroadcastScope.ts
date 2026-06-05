import type { TerraBroadcastOptions } from './terraBroadcast'

const scopeStack: TerraBroadcastOptions[] = []

/**
 * Active broadcast callbacks for nested contract executes (e.g. allowance then swap).
 * {@link useTerraBroadcastMutation} wraps mutations with this scope so service layers
 * do not need an `onPhaseChange` parameter on every entry point (GitLab #305).
 */
export function withTerraBroadcastScope<T>(
  options: TerraBroadcastOptions | undefined,
  fn: () => Promise<T>
): Promise<T> {
  if (options) scopeStack.push(options)
  return fn().finally(() => {
    if (options) scopeStack.pop()
  })
}

/** @internal Resolved by {@link broadcastTerraExecuteContracts} when no explicit options are passed. */
export function getTerraBroadcastScopeOptions(): TerraBroadcastOptions | undefined {
  return scopeStack[scopeStack.length - 1]
}

/** @internal Test-only reset for isolated unit tests. */
export function resetTerraBroadcastScopeForTests(): void {
  scopeStack.length = 0
}
