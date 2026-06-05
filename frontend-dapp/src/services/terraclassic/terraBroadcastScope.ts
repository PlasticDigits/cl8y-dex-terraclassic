import type { TerraBroadcastOptions } from './terraBroadcast'

const activeScopes = new Set<TerraBroadcastOptions>()
let scopedExecutionDepth = 0

function resolveActiveScope(): TerraBroadcastOptions | undefined {
  if (activeScopes.size !== 1) return undefined
  return activeScopes.values().next().value
}

function invokeScoped<T>(fn: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(() => {
    scopedExecutionDepth++
    try {
      return fn()
    } finally {
      scopedExecutionDepth--
    }
  })
}

/**
 * Active broadcast callbacks for nested contract executes (e.g. allowance then swap).
 * {@link useTerraBroadcastMutation} wraps mutations with this scope so service layers
 * do not need an `onPhaseChange` parameter on every entry point (GitLab #305).
 */
export function withTerraBroadcastScope<T>(
  options: TerraBroadcastOptions | undefined,
  fn: () => Promise<T>
): Promise<T> {
  if (!options) return fn()
  activeScopes.add(options)
  return invokeScoped(fn).finally(() => {
    activeScopes.delete(options)
  })
}

/** @internal Resolved by {@link broadcastTerraExecuteContracts} when no explicit options are passed. */
export function getTerraBroadcastScopeOptions(): TerraBroadcastOptions | undefined {
  if (scopedExecutionDepth === 0) return undefined
  return resolveActiveScope()
}

/** @internal Test-only reset for isolated unit tests. */
export function resetTerraBroadcastScopeForTests(): void {
  activeScopes.clear()
  scopedExecutionDepth = 0
}
