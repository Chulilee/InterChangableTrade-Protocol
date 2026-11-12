# Contributing

Thanks for helping build InterChangableTrade-Protocol. Security reviews and
audits are especially welcome.

## Prerequisites

```bash
rustup target add wasm32-unknown-unknown
```

## Workflow

1. Create a branch off `main`.
2. Make your change with accompanying unit tests.
3. Run the checks locally:

   ```bash
   scripts/test.sh
   ```

4. Build the wasm artifacts to confirm they compile for the target:

   ```bash
   scripts/build.sh
   ```

5. Open a pull request describing the change and its rationale.

## Conventions

- Keep contracts `#![no_std]`; only depend on `soroban-sdk`.
- Return typed `#[contracterror]` errors instead of panicking.
- Publish an event for every state transition.
- Follow the existing module layout: `src/lib.rs` for the contract, `src/test.rs`
  for its tests.
- Format with `cargo fmt` and keep `cargo clippy` warning-free.

## Input validation guide

Contracts must validate external inputs early and fail with typed
`#[contracterror]` values (never panics for user input).

### Required checks

- **Amounts, prices, quantities, rates:** reject non-positive values where the
  domain requires `> 0` (or reject negative values where `0` is meaningful).
- **Durations / time ranges:** reject zero durations when an interval must emit
  rewards or progress over time (e.g. `duration_secs > 0`), and reject invalid
  ranges (e.g. `end_time > start_time`).
- **Identifiers:** reject invalid identifiers such as `trade_id == 0` when IDs
  must be positive.
- **Addresses:** validate addresses at contract boundaries. Use a
  `validate_address` helper to reject zero/invalid addresses where applicable
  before persisting state or transferring value.
- **Authorization:** require auth for callers performing privileged or
  state-changing actions and keep authorization checks consistent with the
  contract's trust model.

### Implementation pattern

- Add or reuse a small helper (for example `validate_address`) near the
  contract entry points so checks stay consistent.
- Prefer explicit, descriptive error variants (e.g. `InvalidAmount`,
  `InvalidParameters`, `InvalidTradeId`, `InvalidTimeRange`) over generic
  failures.
- Add unit tests that assert invalid inputs are rejected with the expected
  error for every new or changed validation path.
