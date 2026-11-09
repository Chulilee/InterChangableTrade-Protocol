# Drips Wave Onboarding

This document is for contributors joining the Drips Wave sprint. It explains
what to pick up, how to build and verify, and what "done" means.

## What this repository is

InterChangableTrade-Protocol is a Soroban (Rust) workspace of 16 smart contracts
on Stellar. The core flow today is:

```
asset-registry → asset-registry-backed listings → marketplace → trade-settlement
                                                      ↓
                                                  escrow (deposit accounting)
                                                      ↓
                                              fee-commission → treasury
```

See [docs/architecture.md](architecture.md) and [docs/adr/](adr/README.md) for
the full map. **Do not treat either settlement path as final — ADR-006 picks
`trade-settlement` as the canonical one; migrating the remaining `escrow`
callers is a tracked issue, not a Wave task.**

## What is verifiably true today

- CI is green on `main` at the time of writing. Every PR must keep it green.
- `scripts/check.sh` reproduces CI's gates locally. Run it before opening a PR.
- The workspace builds for `wasm32-unknown-unknown`; each contract is under the
  64 KiB limit.
- Rust/Soroban with `soroban-sdk 22`.

**Not verifiably true today:** production or mainnet readiness, on-chain legal
status, audited cryptography, or complete role-based access control. Do not
write any of these in a PR description, a release note, or a marketing page.

## How to run tests

```bash
scripts/check.sh
```

This runs, in order:

1. `cargo fmt --all -- --check`
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `scripts/build.sh` (release wasm build + `check-size.sh` for the 64 KiB limit)
5. `cargo test --workspace`

Individual crates (faster feedback):

```bash
cargo test -p <crate-name>
```

Examples:

- `cargo test -p marketplace`
- `cargo test -p trade-matching`
- `cargo test --workspace`

## How to build contracts

```bash
scripts/build.sh
```

Artifacts land in `target/wasm32-unknown-unknown/release/*.wasm`. These are
what a deployment flow (`scripts/deploy.sh`) pushes to a network.

## How to submit a PR

1. Branch off `main`: `git checkout -b <scope>/<short-name> origin/main`
   (e.g. `marketplace/self-fill-guard`).
2. Implement the smallest change that satisfies the issue's acceptance criteria.
   If you cannot picture the smallest version, the issue is too big — ask in the
   issue thread before writing code.
3. Keep `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, and `cargo test --workspace`
   green. `scripts/check.sh` is the arbiter.
4. Add tests. Tests are the contract's spec, not decoration. Every contract
   crate has a test module under `src/test.rs` (or `#[cfg(test)] mod test`).
5. Update docs if behavior, configuration, errors, or workflows change.
   `docs/architecture.md` is the source of truth for module layout; `docs/adr/`
   records load-bearing decisions (use 7+ digits for ADR numbers).
6. Open the PR with a clear title, a summary of what changed and why, and a
   closing keyword tied to the issue (`Closes #N`).

## What "done" looks like for a Wave issue

A PR counts as **done** when:

- CI is green on the PR HEAD (all matrix jobs + `integration-tests` + `Benchmarks`
  if the workflow targets your crate).
- `scripts/check.sh` exits 0 locally.
- The issue's acceptance criteria are met, one by one, visibly.
- Tests cover the happy path and at least one failure case.
- No security boundary was widened without an explicit note.
- The PR body explains the mechanism, not just the diff.

A PR is **not done** if any of the following are true:

- You weakened CI to make it pass (whitelisted a clippy, skipped a failing test,
  disabled a check).
- You documented a risk but did not reduce or bound it.
- You changed a public contract signature, error variant, or event without an
  ADR note justifying the break.
- You claimed "production-ready" anywhere in the output.

## What to work on this Sprint (initial wave shortlist)

| # | Title | Labels | Difficulty |
|---|---|---|---|
| #95 | initialize() front-running: add `require_auth` | security, advanced | handled in this PR |
| #88 | `marketplace::check_risk` must return typed error | bug, good-first-issue | refined in this PR |
| #92 | `marketplace::fill_listing` must reject self-fill | bug, good-first-issue | refined in this PR |
| #93 | `calculate_fees` overflow | bug | refined in this PR |
| #96 | duplicate fee recipients | security | refined in this PR |
| #62 | duplicate settlement paths — staged migration | advanced | umbrella; do not assign Wave mass without scope |
| #61 | extract common authorization pattern | intermediate | good Wave project |
| #66 | extract common contract initialization pattern | good-first-issue | good Wave project |
| #67 | standardize storage key patterns | good-first-issue | good Wave project |
| #73 | frontend: shared validation utility | good-first-issue | good Wave project |

## Recommended first steps for a new contributor

1. Read [ARCHITECTURE](architecture.md) (especially "Architecture" and "Open
   integration gaps") and [CONTRIBUTING.md](../CONTRIBUTING.md).
2. Clone the repo, install Rust 1.98.0+, and run `scripts/check.sh` until it
   exits 0.
3. Pick an issue labeled `good-first-issue` small in scope.
4. Comment "picking up #N" on the issue.
5. Open a PR with the template in the `.github/PULL_REQUEST_TEMPLATE.md` if
   present.

## Where this repo is honest about its state

`docs/DRIPS_WAVE_READINESS.md` is the snapshot. The short version: the contracts
build and their tests pass; the settlement path is not yet final; the role
system exists but is decorative today; no independent review has been done.

## Questions / contact

File issues tagged `good-first-issue`, `question`, or `help-wanted`. If you
need to change architecture, propose an ADR rather than an implementation — the
reviewers want the decision auditable before the code exists.
