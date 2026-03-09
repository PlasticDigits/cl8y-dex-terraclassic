# Architecture Overview

CL8Y DEX is a constant-product AMM deployed on Terra Classic. The system comprises three core contracts — Factory, Pair, and Router — plus an extensible hook interface.

## Contract Relationships

```mermaid
graph TD
    GOV[Governance Wallet] -->|owns| FACTORY[Factory]
    FACTORY -->|instantiates| PAIR1[Pair A-B]
    FACTORY -->|instantiates| PAIR2[Pair C-D]
    PAIR1 -->|instantiates| LP1[LP Token CW20]
    PAIR2 -->|instantiates| LP2[LP Token CW20]
    FACTORY -->|SetPairFee / SetPairHooks| PAIR1
    FACTORY -->|SetPairFee / SetPairHooks| PAIR2
    ROUTER[Router] -->|queries factory, forwards swaps| PAIR1
    ROUTER -->|queries factory, forwards swaps| PAIR2
```

## Swap Flow

```mermaid
sequenceDiagram
    participant User
    participant CW20 as Input CW20
    participant Pair
    participant Treasury
    participant Hook
    participant OutCW20 as Output CW20

    User->>CW20: Send { contract: Pair, msg: Swap }
    CW20->>Pair: Receive(Cw20ReceiveMsg)
    Pair->>Pair: Compute k = x * y, deduct fee
    Pair->>OutCW20: Transfer fee to Treasury
    Pair->>OutCW20: Transfer net output to User
    Pair->>Hook: AfterSwap(pair, sender, tokens, amounts, fee)
```

## Key Design Decisions

- **Constant product (x * y = k):** simple, battle-tested AMM invariant.
- **Fee-on-output:** fee is taken from the computed output amount, not the input.
- **Factory-gated governance:** only the Factory can update pair fees and hooks, keeping governance centralized at one address.
- **Code ID whitelist:** the Factory validates that both tokens in a pair were instantiated from whitelisted CW20 code IDs, preventing malicious token contracts.
- **Hook system:** post-swap hooks allow composable integrations (analytics, rewards, etc.) without modifying the core pair logic.

## Directory Layout

```
smartcontracts/
├── contracts/
│   ├── factory/    # Pair registry, governance, code ID whitelist
│   ├── pair/       # AMM logic, LP minting/burning, fee management
│   ├── router/     # Multi-hop routing (single-hop in v1)
│   └── hooks/      # Example hook contracts
├── packages/
│   └── dex-common/ # Shared types, messages, pagination
└── tests/          # Integration test harness
```
