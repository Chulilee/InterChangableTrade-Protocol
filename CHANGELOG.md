# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) where practical.

## [Unreleased]

## [0.1.1] - 2026-11-10

### Added
- `CHANGELOG.md` and a documented release process.
- CI check that fails when `panic!()` or `.unwrap()` appears in contract source files (excluding tests).

### Changed
- Bumped workspace version to `0.1.1`.
- Standardized contract error handling to prefer typed `#[contracterror]` results and avoid panicking in contract source.
- Replaced `panic!`/`unwrap()` usage in contract source paths across affected contracts, including `cross-chain-bridge`, `orderbook`, `risk-management`, `marketplace`, `matching-engine`, `notification`, `liquidity-incentives`, `margining-liquidation`, and `trade-matching`.

### Fixed
- Missing storage/configuration reads in contract code now return typed errors (e.g. `NotInitialized` / not-found variants) instead of unwrapping.

## [0.1.0] - 2026-11-10

### Added
- Initial Soroban smart-contract workspace for InterChangableTrade-Protocol.
- Core protocol contracts (access control, asset registry, escrow, marketplace, matching/orderbook, settlement, risk/margining, governance, notifications, incentives, and cross-chain bridge modules).
- Integration tests, benchmarks, frontend/docs scaffolding, and contract CI workflows.

[Unreleased]: https://github.com/Chulilee/InterChangableTrade-Protocol/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/Chulilee/InterChangableTrade-Protocol/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Chulilee/InterChangableTrade-Protocol/releases/tag/v0.1.0
