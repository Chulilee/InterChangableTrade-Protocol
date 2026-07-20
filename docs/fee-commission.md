# Fee & Commission System

Calculates, collects, and distributes protocol and maker/taker trading fees for
the InterChangableTrade protocol. Fees are configurable, support exemptions, and
can be changed on a governance-controlled schedule.

## Model

Fees are expressed in **basis points** (bps): `10_000 bps = 100%`. A trade of
`notional` units incurs a fee of `notional * fee_bps / 10_000` (integer division,
truncating toward zero).

| Parameter          | Meaning                                                              |
|--------------------|----------------------------------------------------------------------|
| `maker_fee_bps`    | Fee charged to the maker (liquidity-providing) side of a trade.      |
| `taker_fee_bps`    | Fee charged to the taker (liquidity-taking) side of a trade.         |
| `protocol_fee_bps` | Share of every *collected* fee routed to the protocol treasury.      |
| `protocol_treasury`| Address that receives the protocol's cut plus any rounding dust.     |

Each side's trading fee is capped at `MAX_FEE_BPS` (1000 bps = 10%) to guard
against fat-finger governance updates. `protocol_fee_bps` may range `0..=10_000`.

Collected fees accumulate in a per-asset pool held by the contract. When
`distribute_fee` runs, the treasury takes `protocol_fee_bps` of the pool and the
remainder is split among the configured `FeeRecipient`s by their `share_bps`
(which must sum to exactly `10_000`). Any integer-division dust is swept to the
treasury so the pool always zeroes out exactly.

## Lifecycle

```
initialize ─▶ set_recipients ─▶ collect_fee (per trade) ─▶ distribute_fee
                                      │
governance: set_fees / set_exempt / schedule_fee_update ─▶ apply_scheduled_update
```

`collect_fee` and `distribute_fee` move real tokens via the Stellar Asset
Contract (`token::Client`). The payer must have authorized and hold a balance in
`asset`; the contract must have been funded by prior collections before it can
distribute.

## Entry points

| Function                 | Auth        | Description                                                      |
|--------------------------|-------------|------------------------------------------------------------------|
| `initialize`             | once        | Set admin (governance) and initial fee config.                   |
| `calculate_fees`         | none (view) | Fee owed for `(payer, amount, side)`; `0` if payer is exempt.    |
| `collect_fee`            | payer       | Pull the computed fee from `payer` into the per-asset pool.      |
| `distribute_fee`         | none        | Pay the asset pool to treasury + recipients; permissionless.     |
| `set_fees`               | admin       | Replace the active fee config immediately.                       |
| `set_recipients`         | admin       | Set recipients; `share_bps` must sum to `10_000`.                |
| `set_exempt`             | admin       | Add/remove an address from the fee-exemption whitelist.          |
| `schedule_fee_update`    | admin       | Queue a config change to activate at a future ledger timestamp.  |
| `apply_scheduled_update` | none        | Apply the queued change once `activate_at` is reached.           |
| `is_exempt` / `get_config` / `get_recipients` / `get_fee_pool` / `get_pending` | none (view) | Read state. |

`distribute_fee` and `apply_scheduled_update` are permissionless by design: funds
only ever move to pre-configured addresses, and the scheduled change plus its
timing were fixed in advance by governance. This lets a keeper or cron drive them
without holding admin keys.

## Events

| Event        | Topics                        | Data                       |
|--------------|-------------------------------|----------------------------|
| `feecollct`  | `(payer, asset)`              | `fee: i128`                |
| `feedistr`   | `(asset)`                     | `total: i128`              |
| `cfgset`     | `()`                          | `FeeConfig`                |
| `cfgsched`   | `()`                          | `(FeeConfig, activate_at)` |
| `exemptset`  | `(address)`                   | `exempt: bool`             |

## Example

```rust
// Governance initializes: 10 bps maker, 20 bps taker, 30% protocol cut.
client.initialize(&admin, &10, &20, &3_000, &treasury);

// Split the non-protocol remainder 60/40 between two recipients.
client.set_recipients(&vec![
    &env,
    FeeRecipient { address: r1, share_bps: 6_000 },
    FeeRecipient { address: r2, share_bps: 4_000 },
]);

// A taker trades 1_000_000 units of `asset`.
// Fee = 1_000_000 * 20 / 10_000 = 2_000, pulled into the asset pool.
let fee = client.collect_fee(&taker, &asset, &1_000_000, &Side::Taker); // 2_000

// Later, distribute the accumulated pool.
// treasury += 2_000 * 3_000/10_000 = 600; remainder 1_400 → r1=840, r2=560.
client.distribute_fee(&asset);
```

### Exemptions

```rust
client.set_exempt(&market_maker, &true);
client.calculate_fees(&market_maker, &1_000_000, &Side::Taker); // 0
// collect_fee on an exempt payer returns 0 and performs no token transfer.
```

### Scheduled governance change

```rust
// Queue a fee cut to take effect at ledger timestamp 2_000.
client.schedule_fee_update(&5, &10, &2_500, &treasury, &2_000);

// Anyone may apply it once the ledger clock reaches 2_000.
client.apply_scheduled_update(); // errors NotYetActive before then
```

## Rounding

All fee maths use integer division and truncate toward zero. In `distribute_fee`
the treasury receives `protocol_fee_bps` of the pool **plus** the dust left over
after recipient shares are floored, guaranteeing the pool is fully paid out and
reset to `0`. For a pool of `100` split three ways at `3_333 / 3_333 / 3_334` bps
with a `0%` protocol cut, recipients receive `33 / 33 / 33` and the treasury
sweeps the remaining `1`.

## Gas / resource cost estimates

Soroban meters CPU instructions, memory bytes, and ledger read/write entries
rather than a single "gas" figure; the cost of an invocation is dominated by the
number of ledger entries it touches. Approximate footprint per call:

| Function                 | Ledger reads          | Ledger writes         | Notes                                            |
|--------------------------|-----------------------|-----------------------|--------------------------------------------------|
| `calculate_fees`         | 1 (config) + 1 exempt | 0                     | Read-only; cheapest path.                        |
| `collect_fee`            | config + exempt + pool | 1 (pool) + token xfer | One SAC `transfer` (its own read/write entries). |
| `distribute_fee`         | config + recipients + pool | 1 (pool) + `N`+1 token xfers | Cost scales linearly with recipient count `N`.   |
| `set_fees`               | 1 (admin)             | 1 (config)            | Instance storage.                                |
| `set_recipients`         | 1 (admin)             | 1 (recipients)        | Write grows with recipient count.                |
| `set_exempt`             | 1 (admin)             | 1 (exempt)            | Persistent entry per address.                    |
| `schedule_fee_update`    | 1 (admin)             | 1 (pending)           |                                                  |
| `apply_scheduled_update` | 1 (pending)           | 1 (config) + 1 remove |                                                  |

Practical guidance:

- **`distribute_fee` is `O(N)`** in the number of recipients — each recipient adds
  one token transfer. Keep the recipient list small (single digits) to bound the
  per-distribution cost, and distribute per asset only when the pool is worth the
  fixed transfer overhead (batch many `collect_fee`s into one distribution).
- **`collect_fee` cost is dominated by the token transfer.** Exempt payers skip
  the transfer entirely and cost roughly the same as `calculate_fees`.
- Config, recipients, and pending live in **instance** storage; fee pools and
  exemption flags live in **persistent** storage. Neither this module's logic nor
  its tests bump entry TTLs, so long-lived deployments should include TTL
  extension in their operational tooling.

Exact metered values depend on SDK version and network settings; measure against
your target network with `stellar contract invoke --cost` before mainnet use.
