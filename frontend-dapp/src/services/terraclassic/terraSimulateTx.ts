/**
 * Unsigned Cosmos tx bytes for one LCD `POST /cosmos/tx/v1beta1/simulate` (#1360).
 * The gas limit inside this query lets the node finish the execution. It is not the signed fee.
 */
import {
  CosmosBaseV1beta1Coin,
  CosmosCryptoSecp256k1PubKey,
  CosmosTxV1beta1AuthInfo,
  CosmosTxV1beta1Fee,
  CosmosTxV1beta1ModeInfo,
  CosmosTxV1beta1ModeInfo_Single,
  CosmosTxV1beta1SignerInfo,
  CosmosTxV1beta1TxBody,
  CosmosTxV1beta1TxRaw,
  CosmwasmWasmV1MsgExecuteContract,
} from '@goblinhunt/cosmes/protobufs'
import { HYBRID_SWAP_GAS_LIMIT } from './hybridSwapGas'
import type { AutoGasExecuteEntry } from './swapAutoGas'

/** Query ceiling. Same number as the auto-gas cap so a retail simulate can finish. */
export const AUTO_GAS_QUERY_GAS_LIMIT = HYBRID_SWAP_GAS_LIMIT

const EXECUTE_TYPE = '/cosmwasm.wasm.v1.MsgExecuteContract'
const PUBKEY_TYPE = '/cosmos.crypto.secp256k1.PubKey'
const SIGN_MODE_DIRECT = 1

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

/** TS 5.9 types `Uint8Array` as `ArrayBufferLike`. protobuf-es fields want a plain `ArrayBuffer`. */
function protobufBytes(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  const buffer = new ArrayBuffer(bytes.byteLength)
  const copy = new Uint8Array(buffer)
  copy.set(bytes)
  return copy
}

export function base64ToBytes(value: string): Uint8Array<ArrayBuffer> {
  const binary = atob(value)
  const out = new Uint8Array(new ArrayBuffer(binary.length))
  for (let i = 0; i < binary.length; i += 1) out[i] = binary.charCodeAt(i)
  return out
}

export function encodeSwapSimulateTxBytes(input: {
  signer: string
  entries: AutoGasExecuteEntry[]
  pubKeyB64: string
  sequence: bigint
  queryGasLimit?: number
}): Uint8Array {
  const messages = input.entries.map((entry) => {
    const wasm = new CosmwasmWasmV1MsgExecuteContract({
      sender: input.signer,
      contract: entry.contract,
      msg: new TextEncoder().encode(JSON.stringify(entry.msg)),
      funds: (entry.coins ?? []).map((coin) => new CosmosBaseV1beta1Coin({ denom: coin.denom, amount: coin.amount })),
    })
    return { typeUrl: EXECUTE_TYPE, value: wasm.toBinary() }
  })

  const body = new CosmosTxV1beta1TxBody({ messages, memo: '' })
  const pubKey = new CosmosCryptoSecp256k1PubKey({ key: base64ToBytes(input.pubKeyB64) })
  const single = new CosmosTxV1beta1ModeInfo_Single({ mode: SIGN_MODE_DIRECT })
  const modeInfo = new CosmosTxV1beta1ModeInfo({
    sum: { case: 'single', value: single },
  })
  const signerInfo = new CosmosTxV1beta1SignerInfo({
    publicKey: { typeUrl: PUBKEY_TYPE, value: pubKey.toBinary() },
    modeInfo,
    sequence: input.sequence,
  })
  const queryGas = BigInt(input.queryGasLimit ?? AUTO_GAS_QUERY_GAS_LIMIT)
  const fee = new CosmosTxV1beta1Fee({ amount: [], gasLimit: queryGas })
  const auth = new CosmosTxV1beta1AuthInfo({ signerInfos: [signerInfo], fee })
  const raw = new CosmosTxV1beta1TxRaw({
    bodyBytes: protobufBytes(body.toBinary()),
    authInfoBytes: protobufBytes(auth.toBinary()),
    signatures: [new Uint8Array(new ArrayBuffer(0))],
  })
  return raw.toBinary()
}

export function queryGasLimitFromSimulateTx(txBytes: Uint8Array): bigint {
  const raw = CosmosTxV1beta1TxRaw.fromBinary(txBytes)
  const auth = CosmosTxV1beta1AuthInfo.fromBinary(raw.authInfoBytes)
  return auth.fee?.gasLimit ? BigInt(auth.fee.gasLimit) : 0n
}

type AccountRecord = Record<string, unknown>

function asRecord(value: unknown): AccountRecord | null {
  if (value == null || typeof value !== 'object' || Array.isArray(value)) return null
  return value as AccountRecord
}

/** BaseAccount and nested vesting / module wrappers. */
export function accountPubkeyAndSequence(body: unknown): { pubKeyB64: string; sequence: bigint } | null {
  const root = asRecord(body)
  const account = asRecord(root?.account)
  if (!account) return null
  const base = asRecord(account.base_account) ?? account
  const pub = asRecord(base.pub_key) ?? asRecord(account.pub_key)
  const key = pub?.key
  if (typeof key !== 'string' || key.trim() === '') return null
  const seqRaw = base.sequence ?? account.sequence ?? '0'
  try {
    return { pubKeyB64: key, sequence: BigInt(String(seqRaw)) }
  } catch {
    return null
  }
}

export type LcdFetch = (url: string, init?: RequestInit) => Promise<Response>

export async function lcdReadSwapGasUsed(input: {
  lcdUrl: string
  signer: string
  entries: AutoGasExecuteEntry[]
  timeoutMs: number
  fetchImpl?: LcdFetch
  queryGasLimit?: number
}): Promise<unknown> {
  const fetchImpl = input.fetchImpl ?? fetch
  const lcd = input.lcdUrl.replace(/\/$/, '')
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), input.timeoutMs)
  try {
    const accountRes = await fetchImpl(`${lcd}/cosmos/auth/v1beta1/accounts/${input.signer}`, {
      signal: controller.signal,
      headers: { accept: 'application/json' },
    })
    if (!accountRes.ok) return null
    const account = accountPubkeyAndSequence(await accountRes.json())
    if (!account) return null

    const txBytes = encodeSwapSimulateTxBytes({
      signer: input.signer,
      entries: input.entries,
      pubKeyB64: account.pubKeyB64,
      sequence: account.sequence,
      queryGasLimit: input.queryGasLimit,
    })
    const simRes = await fetchImpl(`${lcd}/cosmos/tx/v1beta1/simulate`, {
      method: 'POST',
      signal: controller.signal,
      headers: { accept: 'application/json', 'content-type': 'application/json' },
      body: JSON.stringify({ tx_bytes: bytesToBase64(txBytes) }),
    })
    if (!simRes.ok) return null
    const text = await simRes.text()
    if (!text.trim()) return null
    let parsed: unknown
    try {
      parsed = JSON.parse(text) as unknown
    } catch {
      return null
    }
    return gasUsedField(parsed)
  } catch {
    return null
  } finally {
    clearTimeout(timer)
  }
}

export function gasUsedField(body: unknown): unknown {
  const root = asRecord(body)
  const info = asRecord(root?.gas_info)
  if (!info || !('gas_used' in info)) return null
  return info.gas_used
}
