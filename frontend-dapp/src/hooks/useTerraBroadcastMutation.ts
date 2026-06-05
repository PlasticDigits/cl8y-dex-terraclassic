import { useMutation, type UseMutationOptions, type UseMutationResult } from '@tanstack/react-query'
import { useCallback, useMemo, useState } from 'react'
import { flushSync } from 'react-dom'
import type { TerraBroadcastOptions, TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { withTerraBroadcastScope } from '@/services/terraclassic/terraBroadcastScope'

export type UseTerraBroadcastMutationResult<TData, TVariables, TContext> = UseMutationResult<
  TData,
  Error,
  TVariables,
  TContext
> & {
  phase: TerraBroadcastPhase | null
  pendingTxHash: string | null
}

/**
 * React Query mutation wrapper that tracks Terra sign/broadcast/confirm phases for button copy
 * and in-flight tx hash links (GitLab #305). Service calls inside `mutationFn` automatically
 * receive phase callbacks via {@link withTerraBroadcastScope}.
 */
export function useTerraBroadcastMutation<TData = string, TVariables = void, TContext = unknown>(
  options: Omit<UseMutationOptions<TData, Error, TVariables, TContext>, 'mutationFn'> & {
    mutationFn: (variables: TVariables) => Promise<TData>
  }
): UseTerraBroadcastMutationResult<TData, TVariables, TContext> {
  const [phase, setPhase] = useState<TerraBroadcastPhase | null>(null)
  const [pendingTxHash, setPendingTxHash] = useState<string | null>(null)

  const broadcastOptions = useMemo<TerraBroadcastOptions>(
    () => ({
      onPhaseChange: (nextPhase, ctx) => {
        // Async mutationFn batches updates; flush confirming+hash before pollTx (GitLab #330).
        flushSync(() => {
          setPhase(nextPhase)
          if (nextPhase === 'signing') {
            setPendingTxHash(null)
          } else if (ctx?.txHash) {
            setPendingTxHash(ctx.txHash)
          }
        })
      },
    }),
    []
  )

  const { mutationFn, onSettled, ...rest } = options

  const wrappedMutationFn = useCallback(
    (variables: TVariables) => withTerraBroadcastScope(broadcastOptions, () => mutationFn(variables)),
    [broadcastOptions, mutationFn]
  )

  const mutation = useMutation({
    ...rest,
    mutationFn: wrappedMutationFn,
    onSettled: (...args) => {
      setPhase(null)
      setPendingTxHash(null)
      onSettled?.(...args)
    },
  })

  return { ...mutation, phase, pendingTxHash }
}
