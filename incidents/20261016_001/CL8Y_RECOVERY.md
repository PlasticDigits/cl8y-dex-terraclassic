# CL8Y recovery assessment and interim compensation

**Status:** informational, requested separately from the original publication. This document records a read-only assessment of whether the deployed BSC CL8Y guard can recover the attacker-held CL8Y, the agreed interim compensation from the project's CL8Y reserve, and a long-term migration recommendation. It does not replace the [bridge response](BRIDGE_RECOVERY.md) or the historical theft accounting. No tests/CI are run for this requested documentation-only addition.

**18 October execution update:** The agreed reserve compensation is now [verified as executed](RECOVERY_UPDATE_20261018.md#cl8y-reserve-compensation-completed): **25,452.503948354102807429 CL8Y** reached the designated BSC recovery wallet at **07:38:36 UTC**. This was a reserve-funded transfer of existing v2 tokens; it did not seize the attacker's CL8Y or deploy v3. The assessment and balances at block 128108592 below remain historical snapshots.

The values below were read with read-only `eth_call`/`eth_getCode` requests at BSC block **128108592**. They are captured observations, not independently authenticated consensus proofs, and they can change after that block. The attacker labels follow [REPORT.md](REPORT.md) and [evidence/labels.json](evidence/labels.json).

## Verdict

**No.** The guard system deployed on BSC has no working CL8Y asset-recovery path:

1. No recovery module is deployed. The registered guard modules are blacklist/rate/risk modules only. `AssetRecovery.sol` exists only as source in the separate `CL8Y-Token-SC` repository; it has no deploy script and no BSC deployment record.
2. Even the generic `GuardERC20.execute(address,bytes)` primitive cannot force-move CL8Y v2 from the attacker, because the v2 token's intended guard auto-approval cannot run before the ERC20 allowance is spent. This is described under [Why the forced-transfer path fails](#why-the-forced-transfer-path-fails).
3. CL8Y v2 is not upgradeable (no owner, no proxy, no mint/admin-seizure function). No contract deployed against the existing token can move the attacker's balance.

## Deployed state at block 128108592

| Item | Observed value |
| --- | --- |
| CL8Y v2 token | `0x8F452a1fdd388A45e1080992eFF051b4dd9048d2` |
| GuardERC20 | `0x417580DF7eE35FFA6286255b55B456c992657fB9` |
| Guard authority (AccessManager) | `0x745120275A70693cc1D55cD5C81e99b0D2C1dF57` |
| Guard module set id `keccak256("GUARD_MODULES")` | `0xde7e89610cde9801439d1bfe9e6f7e45999a3d3cda04fc1cc6a25974788e77ed` |
| Registered guard modules | `BlacklistSenderOnly` `0x3009E9de998E62D75f8342109270c9F919F3a885`, `BlacklistReceiverOnly` `0x513375eb5bE203Ad4BA442Cc02C4f36d89932659`, `RiskFlagAuto` `0xbD86F71A1dEd9dc9c123106aB18dF9783F19390a` |
| `[HACKER_A]` CL8Y | `0x46b2ebf5ec9bbbca91fb826a6c13aa7ccdb66f8d`, **25,452.503948354102807429 CL8Y** (`25452503948354102807429` base units) |
| `[HACKER_A]` sender-blacklisted / receiver-blacklisted / risk-flagged | **true / false / false** |
| `[HACKER_A]` allowance to the guard | **0** |
| Designated BSC recovery wallet | `0x21ddb4bcb5f00430cb39d5fea2d382bbeaa076ba`, **0 CL8Y**, not blacklisted |
| Project CL8Y reserve (genesis holder) | `0xFAC4C56258941D445Afda6BB2Fa87b493A65B8a1`, **332,069 CL8Y** |
| CL8Y v2 `totalSupply` | `2858049940015032040561313` = 2,858,049.940015032040561313 CL8Y |
| Guard `execute` target function role | `0` (AccessManager admin role) |

CL8Y v2 was deployed in transaction [`0xd272b27c2a6e6fc7e53b6102040fb802a75416b5218d0433e03a7d366cebc237`](https://bscscan.com/tx/0xd272b27c2a6e6fc7e53b6102040fb802a75416b5218d0433e03a7d366cebc237) at block **54573854**, minting the 3,000,000 CL8Y genesis supply to the reserve wallet `0xFAC4C56258941D445Afda6BB2Fa87b493A65B8a1`. The token has no `owner()` and no EIP-1967 implementation slot; it is not a proxy and has no administrative mint, burn or seize entry point.

## Why the forced-transfer path fails

The intended seizure primitive is:

```
GuardERC20.execute(token, abi.encodeCall(IERC20.transferFrom, (attacker, recipient, amount)))
```

`CL8Y_v2._update` does contain a guard auto-approval:

```solidity
if (msg.sender == address(guard)) {
    _approve(from, address(guard), value);
}
```

That approval is unreachable for a zero-allowance `transferFrom`. In OpenZeppelin ERC20 5.2.0, `transferFrom` calls `_spendAllowance(from, spender, value)` **before** `_transfer` (and therefore before `_update`). With `allowance(attacker, guard) == 0`, the call reverts before the auto-approval can run. Clearing blacklist storage does not change the zero allowance.

The read-only simulation at block 128108592 reproduces this:

- `GuardERC20.execute(...)` as the AccessManager admin reverts `GuardERC20: call failed`.
- The inner `CL8Y_v2.transferFrom(...)` reverts `0xfb8f41b2`, i.e. `ERC20InsufficientAllowance(0x417580DF7eE35FFA6286255b55B456c992657fB9, 0, 25452503948354102807429)`.

This is the same result already captured in [the bridge-response simulation](evidence/bridge-recovery-20261016/cl8y-transfer-simulation.json); the [runtime comparison](evidence/bridge-recovery-20261016/assessment-runtime-comparison.json) confirms the selector `0xfb8f41b2` is `ERC20InsufficientAllowance(address,uint256,uint256)`. The existing `AssetRecovery.t.sol` unit test does not exercise this path: it uses a mock guard and a mock token and pre-approves the guard, so it cannot detect the real allowance ordering. A working design would need either a token that overrides `transferFrom`, an administrative mint/burn/seize function, or a holder signature — none of which exists on CL8Y v2.

## Current containment

`[HACKER_A]` remains sender-blacklisted, so ordinary CL8Y transfers from that address revert. The attacker's 25,452.503948354102807429 CL8Y is effectively frozen and can be neither moved by the attacker nor recovered by the guard. This containment depends on retaining the `BlacklistSenderOnly` entry and its `GUARD_MODULES` registration; see the control-dependency note in [BRIDGE_RECOVERY.md](BRIDGE_RECOVERY.md).

## Interim compensation

The agreed interim response is to leave the attacker's tokens locked and compensate the victim/recovery side with an equal amount of CL8Y from the project's unused CL8Y liquidity reserve:

| Field | Value |
| --- | --- |
| From (unused CL8Y liquidity reserve) | `0xFAC4C56258941D445Afda6BB2Fa87b493A65B8a1` (332,069 CL8Y at the checked block) |
| To (designated BSC recovery wallet) | `0x21ddb4bcb5f00430cb39d5fea2d382bbeaa076ba` |
| Token | CL8Y `0x8F452a1fdd388A45e1080992eFF051b4dd9048d2` |
| Amount | **25,452.503948354102807429 CL8Y** (`25452503948354102807429` base units) |
| Purpose | Match the frozen attacker-held CL8Y and make the victim whole for that amount |
| Status | **Executed on 18 October 2026**, BSC block 128323680; [receipt and later custody snapshot](RECOVERY_UPDATE_20261018.md#cl8y-reserve-compensation-completed) |

The reserve wallet held more than the compensation amount at the assessment block. The [18 October follow-up](RECOVERY_UPDATE_20261018.md) captures the successful transfer and later balances. At its custody snapshot the attacker still held the same CL8Y amount and its `BlacklistSenderOnly` entry remained present.

## Long-term recommendation: migrate to CL8Y v3

Because the existing v2 token has no recovery primitive, the durable fix is a v3 migration rather than any attempt to seize v2:

1. Deploy a new CL8Y v3 token and a migration vault.
2. Let legitimate v2 holders approve the vault and deposit v2 for v3 one-for-one.
3. Exclude the attacker automatically: any v2 transfer from `[HACKER_A]` already reverts under `BlacklistSenderOnly`, so the attacker cannot deposit or migrate.
4. After a migration deadline, allocate the v3 equivalent of the frozen `[HACKER_A]` balance to the recovery wallet, keeping the v3 supply consistent with the migrated v2.
5. Move DEX liquidity and exchange references from v2 to v3.

This is a larger ecosystem action (new token address, liquidity and listing changes) and is therefore recorded as a recommendation, not as an executed change. It is the only on-chain route that puts the excluded attacker value at the recovery address, because the current v2 contract cannot be patched and the guard cannot force a transfer.

## Limitations

- The read-only observations above are pinned to block 128108592 and are not consensus proofs; later state may differ.
- The compensation source is identified as the CL8Y genesis/liquidity reserve wallet that received the genesis mint and held the stated balance. If the project designates a different reserve wallet, this document should be updated.
- No v2 seizure, v3 deployment or token migration is evidenced here. The [18 October reserve compensation](RECOVERY_UPDATE_20261018.md) is separately evidenced alongside the [16 October bridge response](BRIDGE_RECOVERY.md). Receipt at the designated recovery wallet does not establish later delivery to the victim.
- This is not a general security audit of CL8Y v2, the guard, or the AccessManager configuration.
