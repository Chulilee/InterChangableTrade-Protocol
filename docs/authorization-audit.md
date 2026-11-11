# Authorization Audit

Audit of how each contract authorizes privileged actions, recorded as part of
issue #61 ("Extract common authorization pattern into reusable module").

## The model

Authorization across the protocol is **role-based** and anchored in the
`access-control` contract. Contracts do not keep their own privileged
addresses; they read the `access-control` address stored at initialization and
ask it whether the caller holds a role, via the shared `shared-auth` crate
(`contracts/shared-auth`).

Two entry shapes exist, choose per operation:

| Operation class | Check |
| --- | --- |
| Admin-only (config, treasury, risk limits, oracle sources) | entry point takes `caller: Address`; `require_admin(&env, &access_control, &caller)` — caller must hold `ADMIN` |
| Emergency pause (risk-management) | `require_pauser(...)` — caller must hold `PAUSER` |
| Operational (price feeds) | `require_operator(...)` — caller must hold `OPERATOR` |
| Governance execution | `require_governor(...)` — caller must hold `GOVERNOR` |
| Non-privileged (trading, escrow flows) | plain `require_auth()` of the acting party |

`shared-auth::require_role` always calls `caller.require_auth()` before the
role check — Soroban cannot authenticate a hidden invoker, so a caller must
always sign, and the role check runs on top of that signature.

`access-control` itself anchors the hierarchy:

- `ADMIN` is granted to the deployer at initialization and administers the
  other roles.
- Role administration is itself role-gated (`RoleAdmin` mapping, defaulting
  `ADMIN` as admin of `OPERATOR`/`PAUSER`/`GOVERNOR`).
- `renounce_role` only lets an account revoke itself.

## Contract-by-contract audit

| Contract | Pattern (before) | Pattern (after) | Privileged ops |
| --- | --- | --- | --- |
| `access-control` | role-based access control | unchanged | `grant_role`/`revoke_role`/`set_role_admin` — role-admin gated |
| `price-oracle` | access-control via inline `invoke_contract` | access-control via `shared-auth` | `set_oracle_source` (ADMIN), price submission (OPERATOR) |
| `marketplace` | local admin address | **access-control via `shared-auth`** | `set_risk_manager` (ADMIN) |
| `risk-management` | local pauser address | **access-control via `shared-auth`** | `pause_market`/`unpause_market`/`set_limit` (PAUSER) |
| `asset-registry` | local admin address | unchanged (unmigrated) | asset register/remove (local ADMIN) |
| `fee-commission` | local admin (governance) address | unchanged (unmigrated) | `set_fees`, `set_recipients`, `set_exempt`, `schedule_fee_update` |
| `liquidity-incentives` | local admin (governance) address | unchanged (unmigrated) | pool create/fund, rate control |
| `matching-engine` | local admin address | unchanged (unmigrated) | `initialize`-time only; matching is permissionless |
| `notification` | local admin address | unchanged (unmigrated) | send notifications (local ADMIN) |
| `trade-matching` | local admin address | unchanged (unmigrated) | registry/config (local ADMIN) |
| `cross-chain-bridge` | local admin + per-bridge validators | unchanged (unmigrated) | bridge config (local ADMIN) |
| `governance` | local admin address (vestigial) | unchanged (unmigrated) | **none** — the stored admin is never read; proposals are gated by proposer/voter auth and thresholds, not by admin |
| `escrow` | acting-party auth only | unchanged | buyer/seller `require_auth()` |
| `trade-settlement` | acting-party auth only | unchanged | buyer/seller `require_auth()` |
| `margining-liquidation` | acting-party auth only | unchanged | user/owner/liquidator `require_auth()` |
| `orderbook` | acting-party auth only | unchanged | owner `require_auth()` |
| `shared-auth` | — | helper crate | `has_role` / `require_role` / `require_admin` / `require_operator` / `require_pauser` / `require_governor` |

## Why the remaining local-admin contracts were left in place

Issue #61 requires at least two additional contracts migrated to
`access-control` and a common pattern documented; it does not require a
flag-day rewrite of all 16 contracts. The remainder are intentionally left
for follow-up issues, each of which needs its own fixture rework and review.
Recommended order, highest privilege first: `fee-commission` →
`liquidity-incentives` → `asset-registry` → `matching-engine` →
`notification` → `trade-matching` → `cross-chain-bridge`. `governance` needs
no migration (its local admin is write-only and never gates an operation);
its `initialize` should drop the vestigial parameter in a dedicated change.

## Findings

1. **Fragmentation (fixed here for the risk surface that matters).** Only
   `price-oracle` consulted `access-control`; `marketplace` kept a local
   admin and `risk-management` a local pauser — both are on the hot path of
   order flow. Both now authorize through `access-control`.
2. **Vestigial authority.** `governance`'s stored admin gates nothing; users
   can misread it as an emergency back-door. It should be removed, not
   "migrated".
3. **No emergency-pause coupling.** The `PAUSER` role was defined but unused;
   `risk-management` is its first consumer.
4. **Deployer signature vs server authority.** `marketplace` and
   `risk-management` initializers take `(admin, access_control, ...)`:
   `admin` only signs the one-time deploy; ongoing privilege lives entirely
   in `access-control`. Deploy tooling must pass the deployer account as
   `admin` and the deployed `access-control` contract id as
   `access_control`.
5. **`access-control` is a single point of truth, not a single point of
   failure.** Its storage lives on-chain with the same hosting guarantees as
   every other contract; the audit surface for "who can do what" converges to
   one contract plus the migration table above.
