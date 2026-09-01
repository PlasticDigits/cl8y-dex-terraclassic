import { TokenLogo } from '@/components/ui/TokenLogo'
import { tokenAssetInfo } from '@/types'
import { SHARE_LINK_BUTTON_LABEL } from '@/utils/sharePageLinkCopy'
import { getAddressForBlockie, getTokenLogoURI } from '@/utils/tokenDisplay'
import { resolveAllowedTokenLogoUri } from '@/utils/tokenLogoAllowlist'

function logoPropsForToken(tokenId: string): {
  logoURI: string | undefined
  addressForBlockie: string | undefined
  blockieSeed: string | undefined
} {
  const info = tokenAssetInfo(tokenId)
  return {
    logoURI: resolveAllowedTokenLogoUri(getTokenLogoURI(info)),
    addressForBlockie: getAddressForBlockie(info),
    blockieSeed: 'token' in info ? undefined : tokenId,
  }
}

/**
 * Visible Swap Share chrome: **Share {pay logo} → {receive logo}**.
 * Logos are decorative (`alt=""`); the button `aria-label` carries both symbols (#715).
 */
export function SwapSharePairLabel({ payId, receiveId }: { payId: string; receiveId: string }) {
  const pay = logoPropsForToken(payId)
  const receive = logoPropsForToken(receiveId)
  return (
    <span className="share-link-button__pair" aria-hidden="true">
      <span>{SHARE_LINK_BUTTON_LABEL}</span>
      <TokenLogo
        size={16}
        logoURI={pay.logoURI}
        addressForBlockie={pay.addressForBlockie}
        blockieSeed={pay.blockieSeed}
      />
      <span className="share-link-button__arrow">→</span>
      <TokenLogo
        size={16}
        logoURI={receive.logoURI}
        addressForBlockie={receive.addressForBlockie}
        blockieSeed={receive.blockieSeed}
      />
    </span>
  )
}
