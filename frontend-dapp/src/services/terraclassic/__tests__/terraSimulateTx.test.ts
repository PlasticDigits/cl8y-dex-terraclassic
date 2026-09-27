import { describe, expect, it } from 'vitest'

import { HYBRID_SWAP_GAS_LIMIT } from '../hybridSwapGas'
import { autoGasWantedFromUsed, resolveAutoGasLimit } from '../swapAutoGas'
import {
  AUTO_GAS_QUERY_GAS_LIMIT,
  accountPubkeyAndSequence,
  encodeSwapSimulateTxBytes,
  gasUsedField,
  lcdReadSwapGasUsed,
  queryGasLimitFromSimulateTx,
} from '../terraSimulateTx'

const SIGNER = 'terra1sender000000000000000000000000000000001'
const PUBKEY = btoa(String.fromCharCode(...new Uint8Array(33).fill(2)))

function accountBody(sequence = '4') {
  return {
    account: {
      '@type': '/cosmos.auth.v1beta1.BaseAccount',
      address: SIGNER,
      pub_key: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: PUBKEY },
      account_number: '1',
      sequence,
    },
  }
}

const entry = {
  contract: 'terra1token0000000000000000000000000000000001',
  msg: {
    execute_swap_operations: {
      operations: [{ terra_swap: {} }, { terra_swap: {} }],
      max_spread: '0.05',
      minimum_receive: '1',
    },
  },
}

describe('LCD swap simulate (#1360)', () => {
  it('puts the query gas ceiling in the simulate tx and not in the signed decision', () => {
    const bytes = encodeSwapSimulateTxBytes({
      signer: SIGNER,
      entries: [entry],
      pubKeyB64: PUBKEY,
      sequence: 4n,
    })
    expect(queryGasLimitFromSimulateTx(bytes)).toBe(BigInt(AUTO_GAS_QUERY_GAS_LIMIT))
    expect(AUTO_GAS_QUERY_GAS_LIMIT).toBe(HYBRID_SWAP_GAS_LIMIT)
    const signed = resolveAutoGasLimit(1_937_976, 1_910_000)
    expect(signed.gasLimit).toBe(2_325_572)
    expect(BigInt(signed.gasLimit)).not.toBe(queryGasLimitFromSimulateTx(bytes))
  })

  it('reads nested and plain account pubkeys', () => {
    expect(accountPubkeyAndSequence(accountBody())?.sequence).toBe(4n)
    expect(
      accountPubkeyAndSequence({
        account: { base_account: accountBody('9').account, pub_key: undefined },
      })?.sequence
    ).toBe(9n)
    expect(accountPubkeyAndSequence({ account: { sequence: '1' } })).toBeNull()
  })

  it('returns raw gas_used on success and null on transport failure', async () => {
    const calls: Array<{ url: string; body?: string }> = []
    const fetchImpl = async (url: string, init?: RequestInit) => {
      calls.push({ url, body: typeof init?.body === 'string' ? init.body : undefined })
      if (url.includes('/accounts/')) {
        return new Response(JSON.stringify(accountBody()), { status: 200 })
      }
      return new Response(JSON.stringify({ gas_info: { gas_used: '1937976' } }), { status: 200 })
    }

    const used = await lcdReadSwapGasUsed({
      lcdUrl: 'http://lcd.test',
      signer: SIGNER,
      entries: [entry],
      timeoutMs: 1_000,
      fetchImpl,
    })
    expect(used).toBe('1937976')
    expect(autoGasWantedFromUsed(used)).toBe(2_325_572)
    const posted = JSON.parse(calls.find((call) => call.url.endsWith('/simulate'))!.body!) as { tx_bytes: string }
    const queryGas = queryGasLimitFromSimulateTx(Uint8Array.from(atob(posted.tx_bytes), (c) => c.charCodeAt(0)))
    expect(queryGas).toBe(15_000_000n)
  })

  it('fails closed for HTTP errors, empty bodies, zero, negative, and the 147M hostile figure', async () => {
    async function read(sim: Response | 'hang') {
      const fetchImpl = async (url: string, init?: RequestInit) => {
        if (url.includes('/accounts/')) return new Response(JSON.stringify(accountBody()), { status: 200 })
        if (sim === 'hang')
          return new Promise<Response>((_resolve, reject) => {
            init?.signal?.addEventListener('abort', () => reject(new Error('aborted')))
          })
        return sim
      }
      return lcdReadSwapGasUsed({
        lcdUrl: 'http://lcd.test',
        signer: SIGNER,
        entries: [entry],
        timeoutMs: sim === 'hang' ? 20 : 1_000,
        fetchImpl,
      })
    }

    expect(await read(new Response('nope', { status: 500 }))).toBeNull()
    expect(await read(new Response('', { status: 200 }))).toBeNull()
    expect(gasUsedField({})).toBeNull()
    expect(await read(new Response(JSON.stringify({ gas_info: { gas_used: '0' } }), { status: 200 }))).toBe('0')
    expect(autoGasWantedFromUsed('0')).toBeNull()
    expect(await read(new Response(JSON.stringify({ gas_info: { gas_used: '-1' } }), { status: 200 }))).toBe('-1')
    expect(autoGasWantedFromUsed('-1')).toBeNull()
    const hostile = await read(new Response(JSON.stringify({ gas_info: { gas_used: '147000000' } }), { status: 200 }))
    expect(resolveAutoGasLimit(hostile, 1_910_000).gasLimit).toBe(1_910_000)
    expect(await read('hang')).toBeNull()
  })
})
