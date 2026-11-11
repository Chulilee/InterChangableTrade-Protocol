# Architecture

InterChangableTrade-Protocol is a Soroban (Rust) workspace of 16 smart
contracts. Each contract lives under `contracts/<name>/` and can be built and deployed on its own.

## Contracts

| Contract                | Responsibility                                                        |
|-------------------------|-----------------------------------------------------------------------|
| `access-control`        | Role-based access control (ADMIN, OPERATOR, PAUSER, GOVERNOR roles).  |
| `asset-registry`        | Admin-curated list of tokenized assets that are eligible to trade.    |
| `marketplace`           | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `escrow`                | Holds a buyer's deposit per trade; releases to seller or refunds buyer. |
| `trade-settlement`      | Atomic bilateral settlement with retry, batching, and a netting engine. |
| `fee-commission`        | Calculates, collects, and distributes protocol and maker/taker fees.  |
| `liquidity-incentives`  | Rewards liquidity providers over time via a reward-per-liquidity accumulator. |
| `orderbook`             | Minimal flat order list (legacy scaffold; see "Trading surfaces").    |
| `matching-engine`       | Price-time priority order book with partial fills and match simulation. |
| `price-oracle`          | Price feeds with deviation/freshness checks and secondary-source fallback. |
| `margining-liquidation` | Margin accounts, collateral, and liquidation of leveraged positions.  |
| `risk-management`       | Market pause switch and order-size limits; called by the marketplace. |
| `governance`            | Proposals, voting, timelock, and execution into a parameter store.    |
| `cross-chain-bridge`    | Asset locking, validator attestations, and emergency withdrawals for cross-chain transfers. |
| `notification`          | Per-user on-chain notifications with read/unread state and email preferences. |
| `trade-matching`        | Matches listings by category, tags, and price range; ranked trade suggestions. |

`cross-chain-bridge`, `notification`, and `trade-matching` are standalone
additions: they compile and are tested individually but are not referenced by
the trading contracts above yet.

## Typical flow

```
        register asset            create listing            fund escrow
Admin  ───────────────────────▶ Registry   Seller ─────────▶ Marketplace   Buyer ─────────▶ Escrow
                                        │                            │
                                   fill listing                 open trade
                                        ▼                            ▼
                                     Buyer ───────────────────────▶ Settlement
                                                 settle / cancel
                                                      │
                                       release ◀──────┴──────▶ refund
                                          (Escrow moves funds)
```

1. **Registry** — an admin registers the assets and quote tokens that may be traded.
2. **Marketplace** — a seller lists an asset at a fixed price; a buyer fills it
   (the marketplace consults `risk-management` before accepting orders).
3. **Escrow** — the buyer funds an escrow for the agreed amount.
4. **Settlement** — a trade record tracks the exchange; on success the escrow is
   released to the seller, otherwise it is refunded to the buyer.

## Order-book flow

The `matching-engine` maintains a per-market (asset, quote) order book with
price-time priority, partial fills, and match simulation. Executed matches are
emitted as `executed` events; on-chain settlement of matched trades is not yet
wired to `trade-settlement` (see "Open integration gaps").

`orderbook` is an earlier, simpler scaffold (a flat, unscoped order list) kept
for reference; `matching-engine` is the canonical order book going forward.

## Design notes

- Every contract is `#![no_std]` and depends only on `soroban-sdk`
  (`orderbook` predates this convention and is the exception).
- State is keyed with a per-contract `DataKey` enum; listings/trades/escrows use
  auto-incrementing `u64` ids.
- Mutating entry points call `require_auth()` on the relevant party, and every
  state transition publishes an event so off-chain indexers can follow along.
  Event symbols follow one convention (lower snake_case, past tense, ≤ 9
  characters, defined once per contract as `EVT_*`); see the
  [event naming rules](CONTRIBUTING.md#event-naming) and the per-contract
  tables there.
- One-time `initialize()` entry points **require auth from the admin/role
  address passed in** (the `risk-management` pattern): deploy tooling must sign
  with the account it passes as `admin`. Contracts without an admin parameter
  (`price-oracle`, `margining-liquidation`) can only be deployed and
  atomically initialized onto a fresh host; see ADR-006 and the
  [initialize-front-running note](architecture.md#open-integration-gaps).
- Errors are returned via `#[contracterror]` enums rather than panics, so callers
  can use the generated `try_*` client methods.
- Units: fees use basis points (`10_000 bps = 100%`); margin ratios and reward
  accumulators use 18-decimal fixed point (`1e18`).

## Authorization model

Privileged operations authorize the caller through the `access-control`
contract. Contracts do not hold their own privileged addresses; at
initialization each is given the `access-control` contract's address, and
privileged entry points take `caller: Address`, sign with it
(`require_auth()`), and require the caller to hold a role.

The single implementation of that check lives in the shared `shared-auth`
crate (`contracts/shared-auth`): `has_role` plus
`require_role`/`require_admin`/`require_operator`/`require_pauser`/
`require_governor`. `require_admin` is the helper every contract uses for
admin-only operations. Role identifiers (`ADMIN`, `OPERATOR`, `PAUSER`,
`GOVERNOR`) are exposed by `shared-auth` and must match the ones
`access-control` hosts.

Initializers take `(admin, access_control, ...)`: `admin` only signs the
one-time deployment; ongoing privilege lives in `access-control`, where the
deployer maps accounts to roles (`grant_role`/`revoke_role`/`set_role_admin`,
themselves role-gated).

Migrated so far: `price-oracle` (pre-existing), `marketplace`, and
`risk-management`. Remaining local-admin contracts and their migration order
are recorded in `docs/authorization-audit.md`, which also catalogs every
contract's pattern. Non-privileged flows (escrow, settlement, margining,
orderbook) stay on plain acting-party `require_auth()`.

## Decision records

The significant architectural decisions behind this design are recorded in
[docs/adr/](adr/README.md):

- [ADR-001](adr/ADR-001-why-soroban-for-smart-contracts.md) — why Soroban for the contracts
- [ADR-002](adr/ADR-002-why-one-contract-per-module.md) — why one contract per module
- [ADR-003](adr/ADR-003-why-settlement-and-escrow-are-separate.md) — why settlement and escrow are separate (and the duplicated settlement paths tracked in the gaps above)
- [ADR-004](adr/ADR-004-why-matching-engine-replaced-orderbook.md) — why the matching engine is canonical and `orderbook` is legacy
- [ADR-005](adr/ADR-005-why-basis-points-for-fee-calculation.md) — why fees and ratios use basis points / 1e18 fixed point
- [ADR-006](adr/ADR-006-canonical-settlement-path.md) — canonical settlement path and the staged escrow migration plan

## Open integration gaps

The contracts above are individually functional but not yet wired
end-to-end. Known gaps, in rough priority order:

1. **Settlement paths are duplicated.** `trade-settlement` moves tokens itself
   (bilateral transfers, optionally netted per batch) and does not use
   `escrow`, which tracks deposits but never moves tokens. Converging on
   `trade-settlement` (ADR-006) removes this duplication once the migration
   issues land; until then, treat `escrow` as accounting-only and
   `trade-settlement` as authoritative for asset movement.
2. **Matched orders are not settled.** The matching engine's settlement hook is
   a placeholder; orders can be marked `Filled` without any tokens moving.
3. **Fees are not attached to trades.** Nothing calls `fee-commission` when a
   trade settles.
4. **Authorization was fragmented — now converging.** `price-oracle`,
   `marketplace`, and `risk-management` authorize privileged operations
   through `access-control` via the shared `shared-auth` crate (see the
   [authorization model](#authorization-model)); other contracts still keep
   their own local admin addresses. The `PAUSER` role now has its first
   consumer (`risk-management`); `GOVERNOR` remains unused. See
   `docs/authorization-audit.md`.
5. **Governance executes into a vacuum.** Proposals write
   `Parameter(Symbol) -> i128` entries that no contract reads.
6. **Margining now takes real token custody.** `margining-liquidation` pulls
   deposited collateral into the contract via `token::Client`, pays liquidation
   incentives from an unallocated reserve, and values collateral using the
   configured price oracle. See `docs/margining-liquidation.md`.