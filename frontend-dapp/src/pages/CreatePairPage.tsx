import { useState } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { createPair, getWhitelistedCodeIds } from '@/services/terraclassic/factory'
import { queryContract } from '@/services/terraclassic/queries'
import { sounds } from '@/lib/sounds'

function useCodeIdCheck(tokenAddr: string) {
  return useQuery({
    queryKey: ['codeIdCheck', tokenAddr],
    queryFn: async () => {
      if (!tokenAddr || !tokenAddr.startsWith('terra1')) return null
      try {
        const codeIdsResp = await getWhitelistedCodeIds()
        const whitelisted = new Set(codeIdsResp.code_ids)
        const info = await queryContract<{ code_id: number }>(tokenAddr, { contract_info: {} }).catch(() => null)
        if (!info) return { valid: false, reason: 'Could not query contract info' }
        if (!whitelisted.has(info.code_id)) return { valid: false, reason: `Code ID ${info.code_id} is not whitelisted` }
        return { valid: true, reason: null }
      } catch {
        return null
      }
    },
    enabled: tokenAddr.length > 5 && tokenAddr.startsWith('terra1'),
    staleTime: 30_000,
  })
}

export default function CreatePairPage() {
  const address = useWalletStore((s) => s.address)
  const [tokenA, setTokenA] = useState('')
  const [tokenB, setTokenB] = useState('')

  const checkA = useCodeIdCheck(tokenA)
  const checkB = useCodeIdCheck(tokenB)

  const createMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      if (!tokenA.startsWith('terra1')) throw new Error('Token A must be a valid Terra address')
      if (!tokenB.startsWith('terra1')) throw new Error('Token B must be a valid Terra address')
      if (tokenA === tokenB) throw new Error('Token addresses must be different')
      return createPair(address, tokenA, tokenB)
    },
    onSuccess: () => sounds.playSuccess(),
    onError: () => sounds.playError(),
  })

  const isValid = tokenA.length > 0 && tokenB.length > 0 && tokenA !== tokenB
  const hasWhitelistWarning = (checkA.data && !checkA.data.valid) || (checkB.data && !checkB.data.valid)

  return (
    <div className="max-w-[520px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          <h2 className="text-lg font-semibold mb-6 uppercase tracking-wide" style={{ fontFamily: "'Chakra Petch', sans-serif" }}>Create Trading Pair</h2>

          <div className="space-y-4">
            <div>
              <label className="label-neo">Token A Contract Address</label>
              <input
                type="text"
                value={tokenA}
                onChange={(e) => setTokenA(e.target.value)}
                placeholder="terra1..."
                className="input-neo font-mono"
              />
              {checkA.data && !checkA.data.valid && (
                <p className="text-amber-400 text-xs mt-1 uppercase tracking-wide font-semibold">{checkA.data.reason}</p>
              )}
              {checkA.data?.valid && (
                <p className="text-green-400 text-xs mt-1 uppercase tracking-wide font-semibold">Code ID whitelisted</p>
              )}
            </div>

            <div>
              <label className="label-neo">Token B Contract Address</label>
              <input
                type="text"
                value={tokenB}
                onChange={(e) => setTokenB(e.target.value)}
                placeholder="terra1..."
                className="input-neo font-mono"
              />
              {checkB.data && !checkB.data.valid && (
                <p className="text-amber-400 text-xs mt-1 uppercase tracking-wide font-semibold">{checkB.data.reason}</p>
              )}
              {checkB.data?.valid && (
                <p className="text-green-400 text-xs mt-1 uppercase tracking-wide font-semibold">Code ID whitelisted</p>
              )}
            </div>

            {tokenA && tokenB && tokenA === tokenB && (
              <p className="text-red-400 text-sm uppercase tracking-wide font-semibold">Token addresses must be different</p>
            )}

            {hasWhitelistWarning && (
              <div className="alert-warning">
                One or both token code IDs are not whitelisted. The transaction will likely fail.
              </div>
            )}

            <div className="alert-info">
              <p className="mb-2 font-semibold uppercase tracking-wide text-xs" style={{ color: 'var(--ink)' }}>Before creating a pair:</p>
              <ul className="list-disc list-inside space-y-1 text-xs" style={{ color: 'var(--ink-subtle)' }}>
                <li>Both tokens must be valid CW20 contracts</li>
                <li>The token code IDs must be whitelisted by governance</li>
                <li>A pair for these tokens must not already exist</li>
              </ul>
            </div>

            <button
              onClick={() => {
                sounds.playButtonPress()
                createMutation.mutate()
              }}
              disabled={!address || !isValid || createMutation.isPending}
              className={`w-full py-4 font-semibold text-base ${
                !address || !isValid || createMutation.isPending
                  ? 'btn-disabled !w-full !py-4'
                  : 'btn-primary btn-cta !w-full !py-4'
              }`}
            >
              {!address
                ? 'Connect Wallet'
                : createMutation.isPending
                  ? 'Creating Pair...'
                  : 'Create Pair'}
            </button>
          </div>

          {createMutation.isError && (
            <div className="mt-4 alert-error">
              {createMutation.error?.message ?? 'Failed to create pair'}
            </div>
          )}

          {createMutation.isSuccess && (
            <div className="mt-4 alert-success">
              <p className="font-semibold mb-2 uppercase tracking-wide">Pair Created Successfully!</p>
              <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                Transaction:{' '}
                <span className="text-green-400 font-mono text-xs break-all">{createMutation.data}</span>
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
