# Contract Deployment Guide

Deploy the InterChangableTrade-Protocol Soroban contracts to Stellar testnet
or mainnet, verify the deployment, and wire the contracts together.

> The contracts are written in Rust/Soroban and compile to WebAssembly. Deploying
> is a three-step loop: build the wasm, deploy the wasm, call `initialize` once
> per contract (three contracts need no initialization). This guide walks through
> that loop for testnet, which is where every new deployment should land first.

## 1. Prerequisites

- **Rust** via [rustup](https://rustup.rs/). `rust-toolchain.toml` already pins the
  stable channel, the `wasm32-unknown-unknown` target, and the `rustfmt`/`clippy`
  components, so rustup installs the right pieces on first build.
- **Stellar CLI** (`stellar`). See
  <https://developers.stellar.org/docs/tools/cli> for install options. Verify with:

  ```bash
  stellar --version
  ```

- **A funded deployer account.** Create and fund a CLI identity (testnet shown):

  ```bash
  # Generate a new keypair identity named "deployer"
  stellar keys generate deployer --network testnet

  # Print the public address
  stellar keys address deployer

  # Fund it from the testnet friendbot
  stellar keys fund deployer --network testnet
  ```

  Mainnet accounts must be funded with real XLM instead. Never reuse a testnet
  key on mainnet, and never commit secret keys.

## 2. Network configuration

Point the CLI at a network. Testnet is the default network for the examples in
this guide.

```bash
# Testnet (add once; skip if you already have it)
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

# Mainnet (only when explicitly promoting a reviewed build)
stellar network add pubnet \
  --rpc-url https://soroban-mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

Use `--network testnet` on every `stellar` invocation against testnet. Wasm
artifacts are network-agnostic; the network is chosen at deploy/invoke time, not
at build time.

## 3. Build and optimize

```bash
scripts/build.sh
```

This runs `cargo build --release --target wasm32-unknown-unknown` and then
`check-size.sh`, which fails the build if any contract exceeds Soroban's
**64 KiB (65536 byte) wasm limit**. `Cargo.toml` already sets `opt-level = "z"`,
LTO and `strip = "symbols"` in `[profile.release]`, so artifacts come out small;
if one ever exceeds the limit, reduce dependencies or split the contract rather
than bumping the limit.

Artifacts land in `target/wasm32-unknown-unknown/release/<contract_name>.wasm`
(hyphens in the package name become underscores in the file name).

Optionally shrink further with the CLI's optimizer:

```bash
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/asset_registry.wasm
```

Deploy the optimized artifact when you use `contract optimize`; the CLI writes it
next to the input file with a `.optimized.wasm` suffix.

## 4. Deployment order

Deploy dependencies before the contracts that reference them at initialization
time. Contracts without initialization-time references to other contracts may be
deployed in any order, but the order below keeps wiring simple.

| # | Contract | Depends on (at init/wiring time) | Notes |
|---|----------|----------------------------------|-------|
| 1 | `access-control` | — | Roles consulted by `price-oracle`. Deploy first. |
| 2 | `price-oracle` | `access-control` | `initialize` takes the access-control address. |
| 3 | `risk-management` | — | Marketplace wires to it after deployment. |
| 4 | `margining-liquidation` | `price-oracle` | `initialize` takes the oracle address. |
| 5 | `asset-registry` | — | Register tradeable assets after init. |
| 6 | `marketplace` | `risk-management` (post-init) | Call `set_risk_manager` after both exist. |
| 7 | `matching-engine` | — | Canonical order book; `orderbook` is legacy. |
| 8 | `trade-settlement` | — | No `initialize`; deploy only. |
| 9 | `escrow` | — | No `initialize`; deploy only. |
| 10 | `fee-commission` | — | `initialize` sets fees and treasury. |
| 11 | `liquidity-incentives` | — | |
| 12 | `governance` | — | |
| 13 | `cross-chain-bridge` | — | Needs a validator set at init. |
| 14 | `notification` | — | |
| 15 | `trade-matching` | — | |
| 16 | `orderbook` | — | Legacy scaffold; deploy only if still needed. |

## 5. Deploy each contract

`scripts/deploy.sh` wraps `stellar contract deploy`, checks for the wasm
artifact and the CLI, and prints the deployed contract id:

```bash
scripts/deploy.sh access-control testnet deployer
```

Equivalent raw command (same shape for every contract):

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/access_control.wasm \
  --source deployer \
  --network testnet
```

Save each printed contract id (`C...`); later steps reference them. Deploy in the
order from §4 and record the ids, e.g. in an untracked `deployments/testnet.toml`
with one `contract = "C..."` line per contract. Contract ids are not secrets.

### Example: access-control (first contract)

```bash
scripts/deploy.sh access-control testnet deployer
```

Example output (ids differ per deployment):

```text
==> Deploying access-control to testnet as deployer
CAAAA...WXYZ
```

## 6. Initialize contracts

Every `initialize` is callable **exactly once**; a second call returns an
`AlreadyInitialized` error. Call it right after the contract is deployed. All
arguments are positional-ish named flags as shown — check `stellar contract
invoke --help` for your CLI version, and `contracts/<name>/src/lib.rs` for the
authoritative signature.

| Contract | `initialize` arguments |
|---|---|
| `access-control` | `admin: Address` |
| `price-oracle` | `access_control: Address`, `max_deviation_bps: u32`, `freshness_threshold: u64` (seconds), `twap_window: u64` (seconds) |
| `risk-management` | `admin: Address`, `access_control: Address`, `max_order_size: i128` |
| `margining-liquidation` | `initial_margin_ratio: i128`, `maintenance_margin_ratio: i128`, `liquidation_incentive: i128` (all 1e18 fixed point), `price_oracle: Address` |
| `asset-registry` | `admin: Address` |
| `marketplace` | `admin: Address`, `access_control: Address` |
| `matching-engine` | `admin: Address` |
| `governance` | `admin: Address` |
| `fee-commission` | `admin: Address`, `maker_fee_bps: i128`, `taker_fee_bps: i128` (≤ 1000 bps), `protocol_fee_bps: i128` (≤ 10000 bps), `protocol_treasury: Address` |
| `liquidity-incentives` | `admin: Address` |
| `cross-chain-bridge` | `admin: Address`, `validators: Vec<Address>`, `required_attestations: u32` (≤ validator count), `fee: i128` (≥ 0), `fee_address: Address` |
| `notification` | `admin: Address` |
| `trade-matching` | `admin: Address` |
| `escrow` | none — deploy only |
| `trade-settlement` | none — deploy only |
| `orderbook` | none — deploy only (legacy) |

Margin ratio semantics: `initial_margin_ratio` must be strictly greater than
`maintenance_margin_ratio`, both must be positive, and
`liquidation_incentive ≥ 0`. Use 1e18 fixed point, e.g. an initial margin of 10%
is `100000000000000000`. Fee basis points: `10_000 bps = 100%`, and maker/taker
fees are capped at `1_000 bps` (10%).

### Example initializations (testnet)

```bash
ACCESS_CONTROL_ID=C...   # from §5
ORACLE_ID=C...
RISK_ID=C...
DEPLOYER=$(stellar keys address deployer)

# 1. access-control
stellar contract invoke --id "$ACCESS_CONTROL_ID" --source deployer --network testnet -- \
  initialize --admin "$DEPLOYER"

# 2. price-oracle (depends on access-control)
stellar contract invoke --id "$ORACLE_ID" --source deployer --network testnet -- \
  initialize --access-control "$ACCESS_CONTROL_ID" \
             --max-deviation-bps 500 \
             --freshness-threshold 300 \
             --twap-window 600

# 3. risk-management
stellar contract invoke --id "$RISK_ID" --source deployer --network testnet -- \
  initialize --pauser "$DEPLOYER" --max-order-size 1000000
```

(Argument flag spelling depends on CLI version — the examples use the
hyphen-case generated from the Rust parameter names. If an invocation fails with
"unknown argument", run `stellar contract invoke --help` and adjust; the
parameter names in `contracts/<name>/src/lib.rs` are authoritative.)

### Post-initialization wiring

Two loose ends remain after `initialize`:

1. **Marketplace → risk-management.** The marketplace consults the risk manager
   before accepting orders, but it learns the address from a separate call:

   ```bash
   stellar contract invoke --id "$MARKETPLACE_ID" --source deployer --network testnet -- \
     set_risk_manager --risk-manager "$RISK_ID"
   ```

2. **Price oracle sources.** Initialize registers the access control and base
   config; feeds themselves are added after deployment via `set_oracle_source`
   and selected with `set_active_source` (see `contracts/price-oracle`).

Optional follow-ups: set fee recipients (`fee-commission set_recipients`, shares
must sum to 10000 bps), fee exemptions (`set_exempt`), and reward rates
(`liquidity-incentives set_reward_rate`).

## 7. Verify the deployment

For each contract, confirm the id resolves and its state matches what you set.
All of these are read-only (no source account signature required beyond auth for
simulation, which the CLI handles):

```bash
# access-control: deployer should hold ADMIN
stellar contract invoke --id "$ACCESS_CONTROL_ID" --network testnet -- \
  has_role --role ADMIN --account "$DEPLOYER"

# asset-registry: empty list right after init
stellar contract invoke --id "$REGISTRY_ID" --network testnet -- list

# fee-commission: config round-trips
stellar contract invoke --id "$FEE_ID" --network testnet -- get_config

# governance: state of proposal 1 should be the freshly-created proposal
stellar contract invoke --id "$GOV_ID" --network testnet -- get_state --proposal-id 1

# margining-liquidation: account of a fresh user errors with account-not-found,
# which proves the contract and its config storage are live
stellar contract invoke --id "$MARGIN_ID" --network testnet -- check_margin --user "$DEPLOYER"
```

Deployment checklist:

- [ ] Every expected contract id is recorded and resolvable.
- [ ] `initialize` succeeded exactly once per contract (a re-run fails with `AlreadyInitialized`).
- [ ] `marketplace set_risk_manager` points at the deployed `risk-management` id.
- [ ] Oracle sources are configured, or intentionally left unconfigured.
- [ ] A smoke read (`has_role`, `list`, `get_config`) returns sensible output.
- [ ] Testnet only until explicitly promoting to pubnet.

## 8. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `The 'stellar' CLI is not installed` | CLI missing or not on `PATH` | Install from <https://developers.stellar.org/docs/tools/cli>; re-check `stellar --version`. |
| `Wasm not found: target/.../x.wasm` | Contract not built, or hyphen/underscore mix-up | Run `scripts/build.sh`; the file name uses underscores (`asset_registry.wasm`), the package uses hyphens (`asset-registry`). |
| `error: exceeded contract size limit` / tx fails on install | Artifact > 64 KiB | Run `scripts/check-size.sh`; reduce dependencies or split the contract; never raise `opt-level`. |
| Deploy tx rejected for insufficient balance | Deployer not funded | `stellar keys fund deployer --network testnet` (testnet) or top up XLM (mainnet). |
| Invocation fails with `AlreadyInitialized` | `initialize` already called | Not a bug; state is set. Inspect with the read functions in §7. |
| Invocation fails with `account not found` / bad sequence | Wrong network or stale identity | Confirm `--network` matches where you deployed; `stellar keys address deployer`. |
| `initialize` succeeds but another contract can't see the config | Mismatched contract id passed at init | Re-check the ids recorded in §5/§6; cross-contract addresses are baked in at init time. |
| `risk-management` rejects every order | `max_order_size` set too low | Re-set with `set_limit`, or redeploy and reinitialize with a sane limit. |
| Marketplace orders fail preflight | `set_risk_manager` not called or wrong id | Call `set_risk_manager` (§6) with the deployed risk-management id; verify with a test order. |
| `required_attestations` rejected on bridge init | Threshold exceeds validator count | Set `required_attestations ≤ validators.len()`. |
| `InvalidFee` on fee-commission init | maker/taker > 1000 bps, or protocol > 10000 bps | Re-initialize with in-range bps values (see §6 table). |
| Oracle prices always stale | `freshness_threshold` too low or no sources | Add/select a source (`set_oracle_source`, `set_active_source`); raise `freshness_threshold` if acceptable. |
| Built wasm differs between machines | Toolchain drift | Use the pinned `rust-toolchain.toml`; CI builds with the same channel. |
| Mainnet deployment attempted by accident | Wrong `--network` | Simulation and deploy are per-invocation; double-check flags before running against pubnet. |

## 9. After deployment

- Record the deployed ids in `deployments/<network>.toml` (untracked) and share
  them with the team.
- Run `cargo test --workspace` and `scripts/build.sh` on the commit you deployed.
- Note the initialization parameters and verification evidence in the release
  notes for the deployed version.

## See also

- [docs/architecture.md](architecture.md) — how the contracts fit together.
- [docs/adr/](adr/) — why the architecture is shaped this way.
- `scripts/build.sh`, `scripts/deploy.sh`, `scripts/check-size.sh`.
