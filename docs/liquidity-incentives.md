# Liquidity Incentives (LP Rewards)

Rewards liquidity providers for supplying depth. Rewards accrue over time, are
proportional to the liquidity each position contributes, and are claimable at
any point without withdrawing the underlying liquidity.

## Reward model

Accrual uses the **reward-per-liquidity accumulator** — the pattern popularised
by Synthetix `StakingRewards`. Each pool emits `reward_rate` reward tokens per
second, shared pro-rata across all active liquidity. Rather than looping over
every position on each block, the contract maintains a single running
accumulator per pool and a checkpoint per position:

```text
reward_per_token += dt * reward_rate * PRECISION / total_liquidity
earned(position)  = position.liquidity
                  * (reward_per_token - position.reward_per_token_paid)
                  / PRECISION
                  + position.rewards
```

`PRECISION` is `1e18`, a fixed-point scalar that keeps the per-token figure from
truncating to zero when `total_liquidity` is large.

The accumulator is advanced (`sync_pool`) on **every** state change — deposit,
withdraw, claim, or rate change — before the change is applied. Because
accrual is a function of elapsed time and the liquidity present during that
interval, this makes rewards:

- **Time-weighted to the second** — a position earns for exactly as long as its
  liquidity was staked.
- **Volume/share-weighted** — a position's cut of each interval's emission is
  `position.liquidity / total_liquidity` at that moment.
- **Exact under rapid deposit/withdraw** — every deposit/withdraw checkpoints
  the accumulator first, so no interval is ever double-counted or missed.

Emissions only accrue while `total_liquidity > 0`. Time when the pool is empty
is skipped (those tokens stay in the reserve), and emission stops at
`period_finish`.

## Assets

Each pool has two tokens:

- `staking_token` — the LP / depth token providers deposit. Held by the contract
  while staked, returned on withdrawal.
- `reward_token` — what providers earn. Must be funded into the contract via
  `fund_pool` before claims can be paid.

## Positions and tick ranges

A `Position` records `liquidity` plus a `tick_lower` / `tick_upper` price range.
The tick range is tracked as metadata for range-based accounting and reporting;
in this module accrual is driven by `liquidity` alone. A withdrawal to zero
leaves the position open (liquidity `0`) so any settled-but-unclaimed rewards
remain claimable.

## Entry points

| Function               | Auth  | Description                                                          |
|------------------------|-------|----------------------------------------------------------------------|
| `initialize`           | once  | Set the admin (governance) address.                                  |
| `create_pool`          | admin | Create a pool with a reward rate and emission duration.              |
| `fund_pool`            | admin | Transfer reward tokens into the contract's reserve.                  |
| `set_reward_rate`      | admin | Change the rate and extend the period; settles accrual first.        |
| `deposit_liquidity`    | owner | Open a position; pulls `staking_token` in. Returns the position id.  |
| `withdraw_liquidity`   | owner | Reduce a position; returns `staking_token`. Rewards stay claimable.  |
| `claim_rewards`        | owner | Pay out accrued `reward_token` and reset the position's counter.     |
| `view_accrued_rewards` | none  | Rewards owed to a position as of now (read-only).                    |
| `get_pool` / `get_position` | none | Read pool / position state.                                    |

## Events

| Event      | Topics                       | Data                    |
|------------|------------------------------|-------------------------|
| `liqdep`   | `(owner, pool_id)`           | `(position_id, amount)` |
| `liqwth`   | `(owner, position_id)`       | `amount`                |
| `rwclaim`  | `(owner, position_id)`       | `amount`                |
| `poolnew`  | `(pool_id)`                  | `Pool`                  |
| `poolfund` | `(pool_id)`                  | `amount`                |
| `rateset`  | `(pool_id)`                  | `(reward_rate, period_finish)` |

## Sample reward computations

All examples use a pool emitting `reward_rate = 100` reward tokens/sec.

### Single provider

```rust
let pos = client.deposit_liquidity(&alice, &pool_id, &1_000, &-10, &10);
// 100 seconds elapse; Alice is the only provider.
// reward_per_token = 100s * 100 * 1e18 / 1_000 = 1e19
// earned = 1_000 * 1e19 / 1e18 = 10_000
client.view_accrued_rewards(&pos); // 10_000
client.claim_rewards(&alice, &pos); // transfers 10_000, counter → 0
```

### Two providers, pro-rata

```rust
// Alice 1_000 (1/3), Bob 2_000 (2/3); rate 90/sec, 100 seconds → 9_000 total.
client.view_accrued_rewards(&pa); // 3_000
client.view_accrued_rewards(&pb); // 6_000
```

### Rapid deposit / withdraw (exact proration)

```text
rate = 100/sec
t=1000  Alice deposits 1_000  (sole provider)
t=1050  Bob deposits 1_000    (50s elapsed: Alice earns 50*100 = 5_000)
t=1100  Bob withdraws 1_000   (50s elapsed, split 50/50: +2_500 each)
t=1150  measure              (50s elapsed: Alice sole again, +5_000)

Alice = 5_000 + 2_500 + 5_000 = 12_500
Bob   =         2_500          =  2_500   (frozen at withdrawal, still claimable)
```

This is the key edge case: because the accumulator is checkpointed at each of
Bob's actions, his rewards stop the instant his liquidity leaves, and Alice
picks up the full emission again — no manual reconciliation needed.

## Rounding

Per-token maths are scaled by `PRECISION` (`1e18`) and `earned` divides back
down with integer truncation. With realistic liquidity magnitudes the truncated
remainder is negligible (sub-token dust), and any unemitted dust simply remains
in the reward reserve rather than being over-paid. Claims can never pay out more
than a position has genuinely earned.

## Gas / resource cost estimates

Soroban meters CPU instructions, memory, and ledger read/write entries rather
than a single "gas" number; cost is dominated by the ledger entries an
invocation touches. Crucially, the accumulator design makes **every operation
`O(1)` regardless of how many providers share a pool** — there is no per-user
loop.

| Function               | Ledger reads        | Ledger writes             | Notes                                   |
|------------------------|---------------------|---------------------------|-----------------------------------------|
| `view_accrued_rewards` | pool + position     | 0                         | Read-only; cheapest path.               |
| `deposit_liquidity`    | pool + counter      | position + pool + counter + token xfer | One SAC `transfer` in.     |
| `withdraw_liquidity`   | pool + position     | position + pool + token xfer | One SAC `transfer` out.              |
| `claim_rewards`        | pool + position     | position + pool + token xfer | Settles then pays reward token.      |
| `create_pool`          | admin + counter     | pool + counter            | Instance + persistent writes.           |
| `fund_pool`            | admin + pool        | token xfer                | Transfers reward tokens into reserve.   |
| `set_reward_rate`      | admin + pool        | pool                      | Settles accrual before rate change.     |

Practical guidance:

- **Cost is independent of provider count.** A pool with 10 or 10_000 providers
  costs the same per deposit/withdraw/claim — the accumulator absorbs the
  sharing maths.
- **`claim_rewards` does not require withdrawing.** Providers can claim as often
  as they like; each claim is one settle + one token transfer.
- Pools and positions live in **persistent** storage; the admin address and id
  counters live in **instance** storage. Long-lived deployments should extend
  entry TTLs via operational tooling — neither the module nor its tests bump TTLs.

Exact metered values depend on the SDK version and network settings; measure
against your target network with `stellar contract invoke --cost` before mainnet
use.
