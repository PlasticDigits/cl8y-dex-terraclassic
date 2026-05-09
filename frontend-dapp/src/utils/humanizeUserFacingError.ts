/**
 * Single entry for UI copy: chain / contract rules first, then wallet & transport classifiers.
 * See [GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134) (tx),
 * [GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145) (off-chain).
 */

import {
  sanitizeOpaqueErrorMessage,
  tryHumanizeFetchLikeMessage,
  tryHumanizeWalletLikeMessage,
} from './humanizeOffChainError'
import { stripNestedTransactionFailedPrefixes, tryHumanizeTerraTxMessage } from './humanizeTerraTxError'

export function getErrorMessage(err: unknown): string {
  if (err == null) return 'Unknown error'
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message.trim() || 'Unknown error'
  try {
    return String(err)
  } catch {
    return 'Unknown error'
  }
}

export function humanizeUserFacingError(raw: string): string {
  const trimmed = raw.trim()
  if (!trimmed) return 'Something went wrong. Please try again.'

  const tx = tryHumanizeTerraTxMessage(trimmed)
  if (tx) return tx

  const inner = stripNestedTransactionFailedPrefixes(trimmed)

  const wallet = tryHumanizeWalletLikeMessage(inner)
  if (wallet) return wallet

  const fetchLike = tryHumanizeFetchLikeMessage(inner)
  if (fetchLike) return fetchLike

  return sanitizeOpaqueErrorMessage(inner)
}

export function humanizeUserFacingErrorFromUnknown(err: unknown): string {
  return humanizeUserFacingError(getErrorMessage(err))
}
