import type { AminoFeeLike } from '@/utils/extensionSignedFeeGuard'

/** Plain JSON amino doc — extensions may ignore protobuf Coin fee amounts (GitLab #127). */
export function toPlainAminoStdSignDoc(stdDoc: {
  fee?: AminoFeeLike & { payer?: string; granter?: string }
  [key: string]: unknown
}): typeof stdDoc {
  const fee = stdDoc.fee ?? { amount: [], gas: '0' }
  return {
    ...stdDoc,
    fee: {
      gas: String(fee.gas ?? '0'),
      amount: (fee.amount ?? []).map((c) => ({
        denom: String(c.denom),
        amount: String(c.amount),
      })),
      ...(fee.payer ? { payer: fee.payer } : {}),
      ...(fee.granter ? { granter: fee.granter } : {}),
    },
  }
}
