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
- Avoid `panic!()` and `unwrap()` in contract source; use `Result`/`ok_or(...)?` or
  Soroban `panic_with_error!` only where a non-`Result` contract API must trap.
- Publish an event for every state transition.
- Follow the existing module layout: `src/lib.rs` for the contract, `src/test.rs`
  for its tests.
- Format with `cargo fmt` and keep `cargo clippy` warning-free.

## Release Process

This project uses a lightweight release flow with [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
formatting in `CHANGELOG.md`.

1. **Bump the version** in the workspace `Cargo.toml` (`[workspace.package].version`).
   Contract crates inherit this via `version.workspace = true`.
2. **Update `CHANGELOG.md`**:
   - Move relevant entries from `[Unreleased]` into a new version section.
   - Add the release date (`YYYY-MM-DD`) and compare/release links at the bottom.
3. **Run checks** locally (at minimum):

   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   scripts/test.sh
   ```

4. **Commit** the version bump and changelog update.
5. **Create a git tag** for the release:

   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin main --tags
   ```

6. **Create a GitHub release** from the tag with release notes derived from the
   corresponding `CHANGELOG.md` section:

   ```bash
   gh release create vX.Y.Z --title "vX.Y.Z" --notes-file <release-notes>
   ```

7. **Verify** CI is green and published artifacts/tags are correct. For breaking
   contract/API changes, call out migration steps in the changelog and release notes.
