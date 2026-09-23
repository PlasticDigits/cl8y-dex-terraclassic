import {
  CosmosCryptoSecp256k1PubKey,
  CosmwasmWasmV1MsgExecuteContract,
} from '@goblinhunt/cosmes/protobufs'

const LCD_URL = (process.env.VERIFY1328_COLUMBUS_LCD_URL || 'https://lcd.terra-classic.hexxagon.io').replace(/\/$/, '')
const INDEXER_URL = (process.env.VERIFY1328_INDEXER_URL || 'https://indexer.dex.cl8y.com').replace(/\/$/, '')
const CL8Y = process.env.VERIFY1328_CL8Y_TOKEN || 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const UST1 = process.env.VERIFY1328_UST1_TOKEN || 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const ROUTER = process.env.VERIFY1328_ROUTER || 'terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw'
const SENDER = process.env.VERIFY1328_SIMULATE_ADDRESS
const AMOUNT_IN = process.env.VERIFY1328_AMOUNT_IN || '1000000000000000000'
const GAS_LIMIT = 3_000_000n

if (!SENDER) {
  throw new Error('Set VERIFY1328_SIMULATE_ADDRESS to a Columbus-5 address with at least 1 CL8Y.')
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

function routeAssetAddress(assetInfo) {
  return assetInfo?.token?.contract_addr?.toLowerCase()
}

function isPoolOnly(operation) {
  const hybrid = operation.terra_swap?.hybrid
  if (hybrid == null) return true
  if (typeof hybrid !== 'object') return false
  return BigInt(hybrid.book_input || '0') === 0n
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
  // Cosmos SDK TxBody(1: Any), AuthInfo(1: SignerInfo, 2: Fee), and TxRaw(1/2/3).
  const bodyBytes = bytesField(1, encodeAny('/cosmwasm.wasm.v1.MsgExecuteContract', messageBytes))
  const modeInfo = bytesField(1, varintField(1, 1)) // ModeInfo.single = SIGN_MODE_DIRECT
  const signerInfo = concatBytes(
    bytesField(1, encodeAny('/cosmos.crypto.secp256k1.PubKey', publicKeyBytes)),
    bytesField(2, modeInfo),
    varintField(3, sequence)
  )
  const fee = varintField(2, 15_000_000)
  const authInfoBytes = concatBytes(bytesField(1, signerInfo), bytesField(2, fee))
  const emptySignature = new Uint8Array()
  return concatBytes(bytesField(1, bodyBytes), bytesField(2, authInfoBytes), bytesField(3, emptySignature))
}

async function main() {
  const amountIn = BigInt(AMOUNT_IN)
  if (amountIn <= 0n) throw new Error('VERIFY1328_AMOUNT_IN must be a positive integer.')

  const routeQuery = new URLSearchParams({ token_in: CL8Y, token_out: UST1, amount_in: amountIn.toString() })
  const route = await getJson(`${INDEXER_URL}/api/v1/route/solve?${routeQuery}`)
  const operations = route.router_operations
  if (
    route.token_in?.toLowerCase() !== CL8Y.toLowerCase() ||
    route.token_out?.toLowerCase() !== UST1.toLowerCase() ||
    operations?.length !== 2 ||
    operations.some((operation) => !isPoolOnly(operation)) ||
    routeAssetAddress(operations[0]?.terra_swap?.offer_asset_info) !== CL8Y.toLowerCase() ||
    routeAssetAddress(operations[1]?.terra_swap?.ask_asset_info) !== UST1.toLowerCase()
  ) {
    throw new Error(`Current route is not the measured two-hop pool-only shape: ${JSON.stringify(route)}`)
  }

  if (
    routeAssetAddress(operations[0].terra_swap.ask_asset_info) !==
    routeAssetAddress(operations[1].terra_swap.offer_asset_info)
  ) {
    throw new Error('Current route operations are not continuous.')
  }

  const accountResponse = await getJson(`${LCD_URL}/cosmos/auth/v1beta1/accounts/${SENDER}`)
  const account = accountResponse.account
  if (!account?.pub_key?.key) throw new Error('Simulation address has no secp256k1 public key on Columbus-5.')
  const balance = await getCw20Balance(CL8Y, SENDER)
  if (balance < amountIn) throw new Error('Simulation address does not have the requested CL8Y balance.')

  // A tiny positive receive floor keeps this a valid read-only execute simulation. The
  // route and message shape match Swap; no signature or broadcast endpoint is used.
  const executeMsg = {
    send: {
      contract: ROUTER,
      amount: amountIn.toString(),
      msg: Buffer.from(
        JSON.stringify({
          execute_swap_operations: {
            operations: operations.map((operation) => ({
              terra_swap: { ...operation.terra_swap, min_return: '1' },
            })),
            max_spread: '0.01',
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
  const pubKey = new CosmosCryptoSecp256k1PubKey({ key: Buffer.from(account.pub_key.key, 'base64') })
  const txBytes = Buffer.from(encodeSimulationTx(wasmExecute.toBinary(), pubKey.toBinary(), account.sequence)).toString(
    'base64'
  )
  const simulation = await fetch(`${LCD_URL}/cosmos/tx/v1beta1/simulate`, {
    method: 'POST',
    headers: { accept: 'application/json', 'content-type': 'application/json', 'user-agent': 'Mozilla/5.0' },
    body: JSON.stringify({ tx_bytes: txBytes }),
  })
  const body = await simulation.text()
  if (!simulation.ok) throw new Error(`Columbus-5 simulation failed (${simulation.status}): ${body.slice(0, 500)}`)

  const gasUsed = BigInt(JSON.parse(body).gas_info.gas_used)
  if (gasUsed >= GAS_LIMIT) {
    throw new Error(`Measured gas_used ${gasUsed} is not below the ${GAS_LIMIT} gas envelope.`)
  }

  console.log(
    JSON.stringify({
      chain: 'columbus-5',
      measurement: 'read-only LCD simulation; no signature or broadcast',
      amount_in: amountIn.toString(),
      quote_kind: route.quote_kind,
      hops: operations.map((operation) => ({
        offer: routeAssetAddress(operation.terra_swap.offer_asset_info),
        ask: routeAssetAddress(operation.terra_swap.ask_asset_info),
        hybrid: operation.terra_swap.hybrid ?? null,
      })),
      gas_used: gasUsed.toString(),
      gas_limit: GAS_LIMIT.toString(),
      headroom: (GAS_LIMIT - gasUsed).toString(),
    })
  )
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exitCode = 1
})
