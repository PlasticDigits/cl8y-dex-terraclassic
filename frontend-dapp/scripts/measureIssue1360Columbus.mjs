import {
  CosmosCryptoSecp256k1PubKey,
  CosmwasmWasmV1MsgExecuteContract,
} from '@goblinhunt/cosmes/protobufs'

const LCD_URL = (process.env.VERIFY1360_COLUMBUS_LCD_URL || 'https://lcd.terra-classic.hexxagon.io').replace(/\/$/, '')
const CL8Y = process.env.VERIFY1360_CL8Y_TOKEN || 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const CLUNC = process.env.VERIFY1360_CLUNC_TOKEN || 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const KENA = process.env.VERIFY1360_KENA_TOKEN || 'terra1ps34wcgyjjp93hf2wvmt3t7v9ky9xm43yhzf6anq8kyp777egsdq0eua67'
const ROUTER = process.env.VERIFY1360_ROUTER || 'terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw'
const SENDER = process.env.VERIFY1360_SIMULATE_ADDRESS
const AMOUNT_IN = process.env.VERIFY1360_AMOUNT_IN || '50000000000000000'

if (!SENDER) {
  throw new Error('Set VERIFY1360_SIMULATE_ADDRESS to a Columbus-5 account that can simulate a CL8Y send.')
}

async function getJson(url) {
  const response = await fetch(url, { headers: { accept: 'application/json', 'user-agent': 'Mozilla/5.0' } })
  const body = await response.text()
  if (!response.ok) throw new Error(`GET ${url} failed (${response.status}): ${body.slice(0, 300)}`)
  return JSON.parse(body)
}

async function getCw20Balance(token, address) {
  const query = Buffer.from(JSON.stringify({ balance: { address } })).toString('base64')
  const response = await getJson(`${LCD_URL}/cosmwasm/wasm/v1/contract/${token}/smart/${encodeURIComponent(query)}`)
  const decoded = typeof response.data === 'string' ? JSON.parse(Buffer.from(response.data, 'base64').toString()) : response.data
  return BigInt(decoded.balance)
}

function concatBytes(...parts) {
  const result = new Uint8Array(parts.reduce((size, part) => size + part.length, 0))
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.length
  }
  return result
}

function varint(value) {
  let remaining = BigInt(value)
  const bytes = []
  while (remaining > 0x7fn) {
    bytes.push(Number((remaining & 0x7fn) | 0x80n))
    remaining >>= 7n
  }
  bytes.push(Number(remaining))
  return Uint8Array.from(bytes)
}

function varintField(field, value) {
  return concatBytes(varint((field << 3) | 0), varint(value))
}

function bytesField(field, value) {
  return concatBytes(varint((field << 3) | 2), varint(value.length), value)
}

function stringField(field, value) {
  return bytesField(field, Buffer.from(value))
}

function encodeAny(typeUrl, value) {
  return concatBytes(stringField(1, typeUrl), bytesField(2, value))
}

function encodeSimulationTx(messageBytes, publicKeyBytes, sequence) {
  const bodyBytes = bytesField(1, encodeAny('/cosmwasm.wasm.v1.MsgExecuteContract', messageBytes))
  const modeInfo = bytesField(1, varintField(1, 1))
  const signerInfo = concatBytes(
    bytesField(1, encodeAny('/cosmos.crypto.secp256k1.PubKey', publicKeyBytes)),
    bytesField(2, modeInfo),
    varintField(3, sequence)
  )
  const fee = varintField(2, 15_000_000)
  const authInfoBytes = concatBytes(bytesField(1, signerInfo), bytesField(2, fee))
  return concatBytes(bytesField(1, bodyBytes), bytesField(2, authInfoBytes), bytesField(3, new Uint8Array()))
}

function hop(offer, ask) {
  return {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: offer } },
      ask_asset_info: { token: { contract_addr: ask } },
      min_return: '1',
    },
  }
}

async function main() {
  const amountIn = BigInt(AMOUNT_IN)
  if (amountIn <= 0n) throw new Error('VERIFY1360_AMOUNT_IN must be a positive integer.')

  const accountResponse = await getJson(`${LCD_URL}/cosmos/auth/v1beta1/accounts/${SENDER}`)
  const account = accountResponse.account?.base_account ?? accountResponse.account
  const pubKey = account?.pub_key?.key
  if (!pubKey) throw new Error('Simulation account has no secp256k1 public key on Columbus-5.')
  const balance = await getCw20Balance(CL8Y, SENDER)
  if (balance < amountIn) throw new Error('Simulation account does not have the requested CL8Y balance.')

  const executeMsg = {
    send: {
      contract: ROUTER,
      amount: amountIn.toString(),
      msg: Buffer.from(
        JSON.stringify({
          execute_swap_operations: {
            operations: [hop(CL8Y, CLUNC), hop(CLUNC, KENA)],
            max_spread: '0.05',
            minimum_receive: '1',
          },
        })
      ).toString('base64'),
    },
  }

  const wasmExecute = new CosmwasmWasmV1MsgExecuteContract({
    sender: SENDER,
    contract: CL8Y,
    msg: Buffer.from(JSON.stringify(executeMsg)),
    funds: [],
  })
  const key = new CosmosCryptoSecp256k1PubKey({ key: Buffer.from(pubKey, 'base64') })
  const txBytes = Buffer.from(
    encodeSimulationTx(wasmExecute.toBinary(), key.toBinary(), BigInt(account.sequence ?? '0'))
  ).toString('base64')
  const simulation = await fetch(`${LCD_URL}/cosmos/tx/v1beta1/simulate`, {
    method: 'POST',
    headers: { accept: 'application/json', 'content-type': 'application/json', 'user-agent': 'Mozilla/5.0' },
    body: JSON.stringify({ tx_bytes: txBytes }),
  })
  const body = await simulation.text()
  if (!simulation.ok) throw new Error(`Columbus-5 simulation failed (${simulation.status}): ${body.slice(0, 500)}`)

  const gasUsed = BigInt(JSON.parse(body).gas_info.gas_used)
  const gasWanted = (gasUsed * 12n + 9n) / 10n
  if (gasWanted <= gasUsed) {
    throw new Error(`ceil(gas_used × 1.2) (${gasWanted}) is not above gas_used ${gasUsed}.`)
  }

  console.log(
    JSON.stringify({
      chain: 'columbus-5',
      measurement: 'read-only LCD simulation; no signature or broadcast',
      route: 'CL8Y → cLUNC → KENA',
      amount_in: amountIn.toString(),
      gas_used: gasUsed.toString(),
      gas_wanted_1_2x: gasWanted.toString(),
    })
  )
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exitCode = 1
})
