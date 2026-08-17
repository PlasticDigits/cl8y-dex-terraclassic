import { useMemo, useState } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { toastErrorMessage, useOptionalToast } from '@/contexts/toastContextState'
import { useWalletStore } from '@/hooks/useWallet'
import { createPair, getWhitelistedCodeIds } from '@/services/terraclassic/factory'
import { getFactoryConfig } from '@/services/terraclassic/settings'
import { formatTokenAmount } from '@/utils/formatAmount'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import { sounds } from '@/lib/sounds'
import { TxResultAlert } from '@/components/ui'
import { CreatePairTokenField } from '@/components/create/CreatePairTokenField'
import { getCreatePairCw20Addresses, sameCreatePairAddress } from '@/utils/createPairTokenCatalog'
import { getTerraAddressInputError } from '@/utils/terraAddressValidation'
import { UST1_CREATE_PAIR_SECONDARY_NOTICE } from '@/utils/ust1SecondaryMarket'

function useCodeIdCheck(tokenAddr: string) {
  return useQuery({
    queryKey: ['codeIdCheck', tokenAddr],
    queryFn: async () => {
      if (!tokenAddr || !tokenAddr.startsWith('terra1')) return null
      try {
        const codeIdsResp = await getWhitelistedCodeIds()
        const whitelisted = new Set(codeIdsResp.code_ids)
        const info = await getChainContractInfo(tokenAddr).catch(() => null)
        if (!info) return { valid: false, reason: 'Could not query contract info' }
        if (!whitelisted.has(info.code_id))
          return { valid: false, reason: `Code ID ${info.code_id} is not whitelisted` }
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
  const toastApi = useOptionalToast()
  const [tokenA, setTokenA] = useState('')
  const [tokenB, setTokenB] = useState('')
  const catalog = useMemo(() => getCreatePairCw20Addresses(), [])

  const checkA = useCodeIdCheck(tokenA)
  const checkB = useCodeIdCheck(tokenB)

  const factoryConfigQuery = useQuery({
    queryKey: ['factoryConfig'],
    queryFn: getFactoryConfig,
    staleTime: 60_000,
  })

  const pairCreationFeeUluna = factoryConfigQuery.data?.pair_creation_fee_uluna?.trim() ?? '0'
  const pairCreationFeeBn = (() => {
    try {
      return BigInt(pairCreationFeeUluna || '0')
    } catch {
      return 0n
    }
  })()

  const createMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      const tokenAError = getTerraAddressInputError(tokenA)
      if (!tokenA || tokenAError) throw new Error(tokenAError ?? 'Token A address required')
      const tokenBError = getTerraAddressInputError(tokenB)
      if (!tokenB || tokenBError) throw new Error(tokenBError ?? 'Token B address required')
      if (sameCreatePairAddress(tokenA, tokenB)) throw new Error('Token addresses must be different')
      return createPair(address, tokenA, tokenB)
    },
    onSuccess: () => {
      sounds.playSuccess()
      toastApi?.pushToast('success', 'Trading pair created.')
    },
    onError: (error) => {
      sounds.playError()
      toastApi?.pushToast('error', toastErrorMessage(error))
    },
  })

  const tokenAError = getTerraAddressInputError(tokenA)
  const tokenBError = getTerraAddressInputError(tokenB)
  const sameTokens = sameCreatePairAddress(tokenA, tokenB)
  const isValid = Boolean(tokenA) && Boolean(tokenB) && !tokenAError && !tokenBError && !sameTokens
  const hasWhitelistWarning = (checkA.data && !checkA.data.valid) || (checkB.data && !checkB.data.valid)

  return (
    <div className="max-w-[520px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          <div className="mb-6">
            <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Create Trading Pair</h2>
            <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
              Add a new CW20 market once both token contracts are verified and approved.
            </p>
          </div>

          <div className="space-y-4">
            <CreatePairTokenField
              label="Token A"
              selectAriaLabel="Select token A"
              value={tokenA}
              onChange={setTokenA}
              catalog={catalog}
              excludeToken={tokenB}
              codeIdCheck={checkA.data ?? null}
            />

            <CreatePairTokenField
              label="Token B"
              selectAriaLabel="Select token B"
              value={tokenB}
              onChange={setTokenB}
              catalog={catalog}
              excludeToken={tokenA}
              codeIdCheck={checkB.data ?? null}
            />

            {sameTokens && (
              <p className="text-red-400 text-sm uppercase tracking-wide font-semibold">
                Token addresses must be different
              </p>
            )}

            {hasWhitelistWarning && (
              <div className="alert-warning">
                One or both token code IDs are not whitelisted. The transaction will likely fail.
              </div>
            )}

            <div className="alert-info">
              <p className="mb-2 font-semibold uppercase tracking-wide text-xs" style={{ color: 'var(--ink)' }}>
                Before creating a pair:
              </p>
              <ul className="list-disc list-inside space-y-1 text-xs" style={{ color: 'var(--ink-subtle)' }}>
                <li>Both tokens must be valid CW20 contracts</li>
                <li>The token code IDs must be whitelisted by governance</li>
                <li>A pair for these tokens must not already exist</li>
                {pairCreationFeeBn > 0n && (
                  <li>
                    Pair creation fee: {formatTokenAmount(pairCreationFeeUluna, 6)} LUNC (attached from wallet on
                    submit)
                  </li>
                )}
                <li>{UST1_CREATE_PAIR_SECONDARY_NOTICE}</li>
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
              {!address ? 'Connect Wallet To Create' : createMutation.isPending ? 'Creating Pair...' : 'Create Pair'}
            </button>
          </div>

          {createMutation.isError && (
            <div className="mt-4">
              <TxResultAlert type="error" message={createMutation.error?.message ?? 'Failed to create pair'} />
            </div>
          )}

          {createMutation.isSuccess && (
            <div className="mt-4">
              <TxResultAlert type="success" message="Pair Created Successfully!" txHash={createMutation.data} />
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
