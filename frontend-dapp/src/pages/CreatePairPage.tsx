import { useState } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { createPair, getWhitelistedCodeIds } from '@/services/terraclassic/factory'
import { queryContract } from '@/services/terraclassic/queries'

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
  })

  const isValid = tokenA.length > 0 && tokenB.length > 0 && tokenA !== tokenB
  const hasWhitelistWarning = (checkA.data && !checkA.data.valid) || (checkB.data && !checkB.data.valid)

  return (
    <div className="max-w-lg mx-auto">
      <div className="bg-dex-card rounded-2xl border border-dex-border p-6">
        <h2 className="text-lg font-semibold mb-6">Create Trading Pair</h2>

        <div className="space-y-4">
          <div>
            <label className="text-xs text-gray-400 mb-1 block">Token A Contract Address</label>
            <input
              type="text"
              value={tokenA}
              onChange={(e) => setTokenA(e.target.value)}
              placeholder="terra1..."
              className="w-full px-4 py-3 rounded-xl bg-dex-bg border border-dex-border text-white text-sm font-mono focus:outline-none focus:border-dex-accent placeholder-gray-600"
            />
            {checkA.data && !checkA.data.valid && (
              <p className="text-amber-400 text-xs mt-1">{checkA.data.reason}</p>
            )}
            {checkA.data?.valid && (
              <p className="text-green-400 text-xs mt-1">Code ID whitelisted</p>
            )}
          </div>

          <div>
            <label className="text-xs text-gray-400 mb-1 block">Token B Contract Address</label>
            <input
              type="text"
              value={tokenB}
              onChange={(e) => setTokenB(e.target.value)}
              placeholder="terra1..."
              className="w-full px-4 py-3 rounded-xl bg-dex-bg border border-dex-border text-white text-sm font-mono focus:outline-none focus:border-dex-accent placeholder-gray-600"
            />
            {checkB.data && !checkB.data.valid && (
              <p className="text-amber-400 text-xs mt-1">{checkB.data.reason}</p>
            )}
            {checkB.data?.valid && (
              <p className="text-green-400 text-xs mt-1">Code ID whitelisted</p>
            )}
          </div>

          {tokenA && tokenB && tokenA === tokenB && (
            <p className="text-red-400 text-sm">Token addresses must be different</p>
          )}

          {hasWhitelistWarning && (
            <div className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400 text-sm">
              One or both token code IDs are not whitelisted. The transaction will likely fail.
            </div>
          )}

          <div className="p-4 rounded-xl bg-dex-bg/50 border border-dex-border text-sm text-gray-400">
            <p className="mb-2 font-medium text-gray-300">Before creating a pair:</p>
            <ul className="list-disc list-inside space-y-1 text-gray-500">
              <li>Both tokens must be valid CW20 contracts</li>
              <li>The token code IDs must be whitelisted by governance</li>
              <li>A pair for these tokens must not already exist</li>
            </ul>
          </div>

          <button
            onClick={() => createMutation.mutate()}
            disabled={!address || !isValid || createMutation.isPending}
            className="w-full py-4 rounded-xl font-semibold text-base transition-colors bg-dex-accent text-dex-bg hover:bg-dex-accent/80 disabled:bg-dex-border disabled:text-gray-500 disabled:cursor-not-allowed"
          >
            {!address
              ? 'Connect Wallet'
              : createMutation.isPending
                ? 'Creating Pair...'
                : 'Create Pair'}
          </button>
        </div>

        {createMutation.isError && (
          <div className="mt-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
            {createMutation.error?.message ?? 'Failed to create pair'}
          </div>
        )}

        {createMutation.isSuccess && (
          <div className="mt-4 p-4 rounded-xl bg-green-500/10 border border-green-500/20">
            <p className="text-green-400 font-medium mb-2">Pair Created Successfully!</p>
            <p className="text-sm text-gray-400">
              Transaction:{' '}
              <span className="text-green-400 font-mono text-xs break-all">{createMutation.data}</span>
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
