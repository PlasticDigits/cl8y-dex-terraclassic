import Blockies from 'react-blockies'

export interface TokenLogoProps {
  addressForBlockie?: string
  size?: number
  className?: string
}

export function TokenLogo({ addressForBlockie, size = 20, className = '' }: TokenLogoProps) {
  if (!addressForBlockie) return null

  const scale = Math.max(2, Math.ceil(size / 6))
  return (
    <span
      className={`inline-block shrink-0 overflow-hidden rounded-full ${className}`}
      style={{ width: size, height: size }}
    >
      <Blockies seed={addressForBlockie.toLowerCase()} size={6} scale={scale} />
    </span>
  )
}
