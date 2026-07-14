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
