import { useState } from 'react'
import Blockies from 'react-blockies'

export interface TokenLogoProps {
  addressForBlockie?: string
  /** Used when there is no logo and no CW20 address (e.g. native denom for Blockies). */
  blockieSeed?: string
  logoURI?: string
  size?: number
  className?: string
}

export function TokenLogo({ addressForBlockie, blockieSeed, logoURI, size = 20, className = '' }: TokenLogoProps) {
  const [imgFailed, setImgFailed] = useState(false)

  const wrapClass = `inline-block shrink-0 overflow-hidden rounded-full ${className}`
  const wrapStyle = { width: size, height: size }

  if (logoURI && !imgFailed) {
    return (
      <span className={wrapClass} style={wrapStyle}>
        <img
          src={logoURI}
          alt=""
          width={size}
          height={size}
          className="block object-cover"
          onError={() => setImgFailed(true)}
        />
      </span>
    )
  }

  const blockieSeedFinal = addressForBlockie ?? blockieSeed
  if (blockieSeedFinal) {
    const scale = Math.max(2, Math.ceil(size / 6))
    return (
      <span className={wrapClass} style={wrapStyle}>
        <Blockies seed={blockieSeedFinal.toLowerCase()} size={6} scale={scale} />
      </span>
    )
  }

  return null
}
