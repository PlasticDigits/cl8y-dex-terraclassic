import type { PairInfo } from '@/types'

/**
 * First factory pair (EMBER/CORAL) on LocalTerra for limit-order pool-ref integration ([#166](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)).
 *
 * `make test-charts-integration` sets `VITE_LIMIT_ORDER_INTEGRATION_*` from a live factory query when LCD is up.
 * Hardcoded fallbacks are last-resort only; they drift after `make deploy-local`.
 */
const PAIR_ADDR =
  import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS ??
  'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

const LP_TOKEN =
  import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_LP_TOKEN ??
  'terra19ehn7w9qxjhulu766skgequq8qjtpts6gtwekjgkg4t4ezuyhlfqr5ghcp'

const TOKEN0 =
  import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_TOKEN0 ??
  'terra1t7kqn7qlnnh0up2kf2vgkzraa2g52yzgakae2frd9r5w5qmqlr3sm3anq5'

const TOKEN1 =
  import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_TOKEN1 ??
  'terra14n45jftyuhdxvl4t7lve5jsmzx0n92wnph6m6h73m8emsq9p6qqs6a3lmt'

export const LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS = PAIR_ADDR

export const limitOrderIntegrationPairInfo: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: LP_TOKEN,
  asset_infos: [{ token: { contract_addr: TOKEN0 } }, { token: { contract_addr: TOKEN1 } }],
}
