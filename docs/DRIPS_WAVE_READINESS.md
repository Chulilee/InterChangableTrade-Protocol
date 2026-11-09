# Drips Wave Readiness Report

**Date:** 2026-11-09
**HEAD (upstream/main):** 69cecaa (Merge pull request #154 from `Chulilee/fix/contracts-check-defaults`)
**CI for this report:** https://github.com/Chulilee/InterChangableTrade-Protocol/actions/runs/42567348253 (success)
**Workflow command:** https://github.com/Chulilee/InterChangableTrade-Protocol/actions/runs/42566908957 (Benchmarks, pull_request) and https://github.com/Chulilee/InterChangableTrade-Protocol/actions/runs/42566407344 (Benchmarks, pull_request)

## TL;DR

The repository is **wave-ready**, not mainnet-ready. `cargo check`, `cargo
clippy -D warnings`, `cargo fmt --check`, the wasm build, the 64 KiB size
check, and the full test suite all run green in CI and via `scripts/check.sh`.
There is a real, non-trivial contract set, a green pipeline, honest architecture
docs (ADRs), and a large labelled backlog. There are still material correctness
gaps (settlement duplicated, auth flow fragmented, trade-matching not wired to
settlement) and none of those have been fixed in this effort; they are
documented here and in the architecture doc rather than hidden.

## What is ready (verified)

| Area | Evidence |
| --- | --- |
| CI runs to completion (16 crates + integration tests, self-discovering matrix gone, `defaults.run` fix) | `42567348253` (main, 3m3s) |
| `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, wasm release build, size check, tests — all green | `42566908981`, `42566407344` (Benchmarks) |
| Local reproducible gate | `scripts/check.sh` (runs fmt → check → clippy → build → test) |
| Contracts are individually functional and tested under SDK 22 | 16 crates; integration tests run end-to-end in `check-integration-tests` |
| Architecture decisions are recorded (ADR-001..006) | `docs/adr/` |
| Benchmarks have a CI entry (`Benchmarks` workflow) | `42566908957`, `42566407344` |
| Contributor docs + issue-first workflow | `docs/CONTRIBUTING.md`, `CONTRIBUTING.md` |

### Changes landed for this report

These were implemented in this readiness pass, in this PR:

- **Security: initialize() now requires auth** — every one-time `initialize` that
  takes an admin/role address now calls `admin.require_auth()` first, closing
  the initialize front-running class of bugs (issue #95).
  Contracts touched: `access-control`, `asset-registry`, `fee-commission`,
  `governance`, `liquidity-incentives`, `marketplace`, `matching-engine`,
  `notification`, `trade-matching`, `cross-chain-bridge`.
  `price-oracle` and `margining-liquidation` take no admin parameter and are
  documented as remaining gap #1 below.
- **Typed error instead of panic** — `marketplace::fill_listing()` now rejects
  self-fills (`Error::SelfFill`) and rejects `create_listing` calls when the
  risk manager is missing or unwired (`Error::RiskManagerNotConfigured`).
  `marketplace::create_listing` still rejects non-positive amounts/prices.
- **Overflow protection** — `fee-commission::calculate_fees` uses
  `checked_mul` and returns a typed `InvalidFee` error on overflow instead of
  wrapping silently.
- **Duplicate fee recipients rejected** —
  `fee-commission::validate_shares` rejects duplicate addresses
  (`Error::DuplicateRecipient`, #96).
- **Canonical settlement decision** — ADR-006 documents `trade-settlement` as
  the authority for atomic trade transfers and `escrow` as scope-limited
  buyer-deposit accounting; architecture doc updated to match.
- **Reproducible workspace gate** — `scripts/check.sh` mirrors CI locally.

## What is NOT ready (honest)

1. **Settlement duplication is still in the tree.** `trade-settlement` moves
   tokens directly; `escrow` records deposits without moving tokens. ADR-006
   names a winner and a staged migration, but no caller has been migrated
   yet. No feature should depend on which path executes a trade until that
   migration lands (issue #62).
2. **The matching engine's settlement hook is a placeholder.** A trade can be
   recorded and `Filled` with no tokens ever moving (#108, #61).
3. **Authorization is fragmented.** Only `price-oracle` and `risk-management`
   consult `access-control`; every other contract keeps a local admin and the
   access-control `OPERATOR`/`PAUSER`/`GOVERNOR` roles are unused.
4. **Fees are not attached to settlements.** `fee-commission` computes fees
   but nothing invokes it from a settlement path.
5. **Governance writes `Parameter(Symbol) -> i128` entries nothing reads.**
   Proposals can pass and change nothing (#114 in the backlog).
6. **Initialization front-running in contracts with no admin parameter.**
   `margining-liquidation::initialize` and `price-oracle::initialize` take no
   admin address and therefore cannot enforce `require_auth`; deploy and
   initialize must be a single atomic tx (#1 above — residual).
7. **`escrow`'s release/refund semantics leak into the token path** once
   settlement is canonical; before migration it is undefined which contract
   moves tokens for a filled listing. This ambiguity is the #1 contributor
   confusion risk and is explicitly called out in ADR-006.
8. **Order books are O(n)** and market-size caps are not enforced; depth can
   grow unbounded (issues #134, #133, #99).
9. **No external audit.** No external reviewer has signed off on this code.
10. **No mainnet verification.** Nothing deployed on Stellar Mainnet; delivery
    paths, fees, and failure semantics are test-only.

## Recommended order of work before funding/scaling

| # | Action | Type | Why |
| --- | --- | --- | --- |
| 1 | Decide and document the interim settlement contract on every marketplace<br>flow (see ADR-006) and add a top-level pointer in README | docs | Biggest confusion source |
| 2 | Merge `fix/github-wave-prep` (security fixes) and close #95, #88, #92, #93, #96 | code | Admin front-running and self-fill are the worst open classes |
| 3 | Migrate `escrow` accounting onto `trade-settlement` or remove its token-moving surface | code | Removes the duplicated atomicity |
| 4 | Wire `trade-settlement` completion to `fee-commission::collect_fee` | code | Fees exist but no one calls them |
| 5 | External review of the three paths that move tokens (`trade-settlement`, `escrow`, `margining-liquidation`) | review | No amount of CI substitutes for an external review |
| 6 | Grant the `OPERATOR`/`PAUSER`/`GOVERNOR` roles meaning by plumbing access-control into matching-engine, marketplace, fee-commission | code | Makes access-control load-bearing, not decorative |
| 7 | Benchmarks regression budget and rollback procedure | infra | Only then does the "Is faster than X" claim matter |
| 8 | Mainnet preview with dummy funds and documented rollback | deploy | Nothing advertised as production until this runs |

## Risk level for Drips Wave participation

**Moderate.** There is a real, testable, sellable story (green CI, honest
roadmap, labelled backlog, Rust/Soroban skill gaps to fill), but the project
has not been externally audited, settlement semantics are not yet
authoritative across the two paths, and a Wave contributor can easily commit
to a misunderstanding of "the settlement flow" unless we scope issues tightly.

Mitigations that make Wave participation reasonable:

- Every issue links directly to the contract file(s), the expected behavior,
  and a reproducible local command (`scripts/check.sh`).
- The Wave doc (`docs/DRIPS_WAVE.md`) tells contributors what "done" looks
  like, and explicitly forbids over-claiming "production-ready" or "mainnet-ready".
- No issue is accepted for a Wave sprint if its fix is blocked by the
  unresolved settlement-path ambiguity; those are split into ADRs or future
  milestones, not Wave work.

## Decision

Green-light a Drips Wave with the constraint above: contributor sprint on a
green, honest backlog; no production/mainnet claims until #1–#8 are done and
externally reviewed.
