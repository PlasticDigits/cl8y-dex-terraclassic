import { useMutation, type UseMutationOptions, type UseMutationResult } from '@tanstack/react-query'
import { useCallback, useMemo, useReducer } from 'react'
import { flushSync } from 'react-dom'
import { toastErrorMessage, useOptionalToast } from '@/contexts/toastContextState'
import type { TerraBroadcastOptions, TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { withTerraBroadcastScope } from '@/services/terraclassic/terraBroadcastScope'

type BroadcastUiState = {
  phase: TerraBroadcastPhase | null
  pendingTxHash: string | null
}

type BroadcastUiAction = { type: 'phase'; phase: TerraBroadcastPhase; txHash?: string } | { type: 'reset' }

function broadcastUiReducer(state: BroadcastUiState, action: BroadcastUiAction): BroadcastUiState {
  switch (action.type) {
    case 'reset':
      return { phase: null, pendingTxHash: null }
    case 'phase':
      if (action.phase === 'signing') {
        // onMutate reset clears stale hashes; keep in-flight hash across nested broadcasts (allowance → send).
        return { phase: action.phase, pendingTxHash: state.pendingTxHash }
      }
      if (action.phase === 'confirming') {
        return {
          phase: action.phase,
          pendingTxHash: action.txHash ?? state.pendingTxHash,
        }
      }
      return { phase: action.phase, pendingTxHash: state.pendingTxHash }
    default:
      return state
  }
}

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
export type TerraBroadcastToastOptions<TData> = {
  /** Floating toast on success (GitLab #351). Inline TxResultAlert remains on the form. */
  toastSuccess?: string | ((data: TData) => string)
  /** Default true — surfaces humanized errors as floating toasts. */
  toastOnError?: boolean
}

export function useTerraBroadcastMutation<TData = string, TVariables = void, TContext = unknown>(
  options: Omit<UseMutationOptions<TData, Error, TVariables, TContext>, 'mutationFn'> & {
    mutationFn: (variables: TVariables) => Promise<TData>
  } & TerraBroadcastToastOptions<TData>
): UseTerraBroadcastMutationResult<TData, TVariables, TContext> {
  const [{ phase, pendingTxHash }, dispatchBroadcastUi] = useReducer(broadcastUiReducer, {
    phase: null,
    pendingTxHash: null,
  })

  const broadcastOptions = useMemo<TerraBroadcastOptions>(
    () => ({
      onPhaseChange: (nextPhase, ctx) => {
        // Atomic phase+hash commit before pollTx; flushSync paints confirming+TX link (GitLab #305/#330).
        flushSync(() => {
          dispatchBroadcastUi({ type: 'phase', phase: nextPhase, txHash: ctx?.txHash })
        })
      },
    }),
    []
  )

  const toastApi = useOptionalToast()
  const { mutationFn, onMutate, onSettled, onSuccess, onError, toastSuccess, toastOnError = true, ...rest } = options

  const wrappedMutationFn = useCallback(
    (variables: TVariables) => withTerraBroadcastScope(broadcastOptions, () => mutationFn(variables)),
    [broadcastOptions, mutationFn]
  )

  const mutation = useMutation({
    ...rest,
    mutationFn: wrappedMutationFn,
    onMutate: (...args) => {
      dispatchBroadcastUi({ type: 'reset' })
      // Preserve the caller's onMutate context (react-query forwards it to onError/onSettled).
      return onMutate?.(...args) as TContext | Promise<TContext>
    },
    onSuccess: (...args) => {
      if (toastApi && toastSuccess != null) {
        const msg = typeof toastSuccess === 'function' ? toastSuccess(args[0]) : toastSuccess
        if (msg) toastApi.pushToast('success', msg)
      }
      onSuccess?.(...args)
    },
    onError: (...args) => {
      if (toastApi && toastOnError) {
        toastApi.pushToast('error', toastErrorMessage(args[0]))
      }
      onError?.(...args)
    },
    onSettled: (...args) => {
      dispatchBroadcastUi({ type: 'reset' })
      onSettled?.(...args)
    },
  })

  return { ...mutation, phase, pendingTxHash }
}
