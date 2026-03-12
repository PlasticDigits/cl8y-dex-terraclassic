import { useState } from 'react'
import Blockies from 'react-blockies'

export interface TokenLogoProps {
  addressForBlockie?: string
  logoURI?: string
  size?: number
  className?: string
}

export function TokenLogo({ addressForBlockie, logoURI, size = 20, className = '' }: TokenLogoProps) {
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

  if (addressForBlockie) {
    const scale = Math.max(2, Math.ceil(size / 6))
    return (
      <span className={wrapClass} style={wrapStyle}>
        <Blockies seed={addressForBlockie.toLowerCase()} size={6} scale={scale} />
      </span>
    )
  }

  return null
}
