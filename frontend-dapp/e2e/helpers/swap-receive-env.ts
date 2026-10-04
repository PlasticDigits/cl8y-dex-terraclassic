/** Synthetic contracts for the fully mocked #1256 browser tests. Never deploy these pins. */
export const SWAP_RECEIVE_1256_FIXTURES = {
  factory: 'terra1qyqszqgpqyqszqgpqyqszqgpqyqszqgp5hm70u',
  router: 'terra1qgpqyqszqgpqyqszqgpqyqszqgpqyqsz9namy2',
  mapper: 'terra1qvpsxqcrqvpsxqcrqvpsxqcrqvpsxqcryru6wt',
  treasury: 'terra1qszqgpqyqszqgpqyqszqgpqyqszqgpqyynmlfh',
  clunc: 'terra1q5zs2pg9q5zs2pg9q5zs2pg9q5zs2pg99r67rk',
  custc: 'terra1qcrqvpsxqcrqvpsxqcrqvpsxqcrqvpsx58umgq',
}

export const SWAP_RECEIVE_1256_VITE_ENV = {
  VITE_UST1_TOKEN_ADDRESS: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  VITE_CL8Y_TOKEN_ADDRESS: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  VITE_FACTORY_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.factory,
  VITE_ROUTER_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.router,
  VITE_WRAP_MAPPER_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.mapper,
  VITE_TREASURY_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.treasury,
  VITE_LUNC_C_TOKEN_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.clunc,
  VITE_USTC_C_TOKEN_ADDRESS: SWAP_RECEIVE_1256_FIXTURES.custc,
}
