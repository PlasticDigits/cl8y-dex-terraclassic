import type { AssetInfo } from '@/types'
import { TokenLogo } from './TokenLogo'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'

export interface TokenDisplayProps {
  info: AssetInfo | null
  size?: number
  className?: string
}

export function TokenDisplay({ info, size = 18, className = '' }: TokenDisplayProps) {
  const { displayLabel, addressForBlockie } = useTokenDisplayInfo(info)

  return (
    <span className={`inline-flex items-center gap-1.5 ${className}`}>
      <TokenLogo addressForBlockie={addressForBlockie} size={size} />
      <span>{displayLabel}</span>
    </span>
  )
}
