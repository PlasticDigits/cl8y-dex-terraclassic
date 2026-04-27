import profilesDoc from './profiles.json' with { type: 'json' }

export type ActionKind =
  | 'router_multihop'
  | 'pair_swap'
  | 'hybrid_swap'
  | 'limit_order'
  | 'add_liquidity'
  | 'remove_liquidity'

export interface ProfileConfig {
  id: string
  weights: Record<ActionKind, number>
}

export type ProfilesFile = {
  meanInterTxSeconds: number
  profiles: ProfileConfig[]
}

const file = profilesDoc as ProfilesFile

export function loadProfiles(): ProfilesFile {
  if (!file.profiles || file.profiles.length !== 5) {
    throw new Error('profiles.json must define exactly five profiles.')
  }
  for (const p of file.profiles) {
    const kinds: ActionKind[] = [
      'router_multihop',
      'pair_swap',
      'hybrid_swap',
      'limit_order',
      'add_liquidity',
      'remove_liquidity',
    ]
    let sum = 0
    for (const k of kinds) {
      const w = p.weights[k] ?? 0
      if (w <= 0) throw new Error(`Profile ${p.id}: weight for ${k} must be > 0`)
      sum += w
    }
    const swapish =
      (p.weights.router_multihop ?? 0) +
      (p.weights.pair_swap ?? 0) +
      (p.weights.hybrid_swap ?? 0) +
      (p.weights.limit_order ?? 0)
    if (swapish < 0.25) {
      throw new Error(`Profile ${p.id}: combined swap/limit weight must stay trader-like (>= 0.25).`)
    }
    if (Math.abs(sum - 1) > 1e-6) {
      throw new Error(`Profile ${p.id}: weights must sum to 1 (got ${sum})`)
    }
  }
  return file
}

export function pickActionKind(profile: ProfileConfig, roll: number): ActionKind {
  const kinds = Object.keys(profile.weights) as ActionKind[]
  let acc = 0
  for (const k of kinds) {
    acc += profile.weights[k] ?? 0
    if (roll < acc) return k
  }
  return kinds[kinds.length - 1]!
}
