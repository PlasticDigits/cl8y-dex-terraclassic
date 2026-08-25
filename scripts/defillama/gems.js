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

const INDEXER_DAILY_URL = 'https://indexer.dex.cl8y.com/api/v1/defillama/daily'

/** 2026-05-01 00:00:00 UTC — first day dimension adapters may request. */
const ADAPTER_START = 1777593600

function isGemAddress(addr) {
  return COLUMBUS5_GEM_ADDRESSES.includes(String(addr || '').toLowerCase())
}

module.exports = {
  COLUMBUS5_GEM_ADDRESSES,
  COLUMBUS5_FACTORY,
  INDEXER_DAILY_URL,
  ADAPTER_START,
  isGemAddress,
}
