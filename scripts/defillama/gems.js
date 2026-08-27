'use strict'

/**
 * Columbus-5 soft-launch gem CW20s — same set as
 * `COLUMBUS5_GEM_ADDRESSES` in frontend-dapp/src/utils/pairCatalogRank.ts
 * and indexer/src/indexer/defillama.rs (GitLab #562 / #631).
 *
 * Volume / pair-linked fees exclude pairs whose either leg is in this set.
 * TVL includes gem pool reserves by default (real locked balances).
 */
const COLUMBUS5_GEM_ADDRESSES = [
  'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94', // EMBER
  'terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena', // CORAL
  'terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr', // JADE
  'terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9', // ONYX
  'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc', // RUBY
  'terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme', // TOPAZ
  'terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z', // QUARTZ
  'terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs', // PEARL
].map((a) => a.toLowerCase())

const COLUMBUS5_FACTORY =
  'terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea'

/** 1:1 wrap CW20s of native uluna / uusd (REGISTRY.md). Llama TVL maps these to those denoms. */
const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'

const WRAP_TO_NATIVE = {
  [CLUNC]: 'uluna',
  [CUSTC]: 'uusd',
}

/** UST1 unstablecoin + USTR reserve (REGISTRY.md). Not gems. */
const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const UST1_WINDOW = 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2'
const VFDUSD = 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3'

const INDEXER_DAILY_URL = 'https://indexer.dex.cl8y.com/api/v1/defillama/daily'

/**
 * First UTC day GET /api/v1/defillama/daily returns 200 on Coolify (GitLab #687).
 * 2026-08-17 00:00:00 UTC. Earlier days 404 — do not move Llama `start` earlier.
 * Unix seconds for `?timestamp=`; ISO for dimension-adapters `start`.
 */
const ADAPTER_START = 1786924800
const ADAPTER_START_ISO = '2026-08-17'

function isGemAddress(addr) {
  return COLUMBUS5_GEM_ADDRESSES.includes(String(addr || '').toLowerCase())
}

module.exports = {
  COLUMBUS5_GEM_ADDRESSES,
  COLUMBUS5_FACTORY,
  CLUNC,
  CUSTC,
  WRAP_TO_NATIVE,
  UST1,
  USTR,
  UST1_WINDOW,
  VFDUSD,
  INDEXER_DAILY_URL,
  ADAPTER_START,
  ADAPTER_START_ISO,
  isGemAddress,
}
