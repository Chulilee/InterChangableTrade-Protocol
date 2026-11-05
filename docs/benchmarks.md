# Benchmarks

ICTP contracts are metered by Soroban on two axes per invocation: **CPU
instructions** and **memory bytes** (plus ledger entry reads/writes and
events, which the harness does not report — they are dominated by the same
paths). A 2-5x increase in the cost of a common operation (order placement,
settlement, vote finalization) would price small traders out. The
`benchmarks` crate exists to catch those regressions early.

## What is benchmarked

Eight scenarios covering the three critical paths from issue #53:

| benchmark | path | what it measures |
| --- | --- | --- |
| `matching-engine/place_order-warm-book(32-asks)` | order placement / book insertion | insert a single order against a 32-ask book across 4 price levels |
| `matching-engine/match_orders-sweep(32-asks)` | order matching | an aggressive buy sweeping 32 resting sells |
| `trade-settlement/settle_trade` | settlement (single) | one atomic bilateral trade with two SAC transfers |
| `trade-settlement/settle_batch_netted-8` | settlement (batch + netting) | 8 trades netted between two parties; netting collapsed 16 transfers into 2 (measured) |
| `governance/propose` | governance proposal lifecycle | creating a proposal |
| `governance/finalize_9_votes` | vote counting | base-case finalize with 9 votes |
| `governance/finalize_39_votes` | vote counting (scaling) | finalize cost growth from 9 to 39 votes (~1.9x, near-linear) |
| `governance/execute_proposal` | governance execution | executable path after threshold is met |

## Measured output (example)

```
benchmark                                   instructions   memory_bytes
------------------------------------------ ------------- --------------
matching-engine/place_order-warm-book(32-asks)        639353         173835
matching-engine/match_orders-sweep(32-asks)       6675181         311877
trade-settlement/settle_trade                     521326          85936
trade-settlement/settle_batch_netted-8           1398986         189230
governance/propose                                124385          21804
governance/finalize_9_votes                        78332          11958
governance/finalize_39_votes                      149753          26838
governance/execute_proposal                        85093          14198
```

Numbers are the same across machines — they are host-metered model values,
not wall-clock timings (release mode). They will differ slightly if
`soroban-sdk` is upgraded, so baselines should be refreshed after SDK bumps.

## Running the benchmarks

```bash
# print the report (uses the workspace + release profile)
cargo run --release -p benchmarks

# fail the run on a regression (exit code 1) — used by CI
cargo run --release -p benchmarks -- --check

# regenerate the baseline after intentional cost changes (e.g. repaying debt)
cargo run --release -p benchmarks -- --update-baseline
```

## Reading the results

`--check` compares instruction counts against `benchmarks/baselines.txt`:

- `PASS` — cost is at most `REGRESSION_THRESHOLD` (default **1.25x**) of baseline.
  Override with `BENCHMARK_REGRESSION_THRESHOLD=1.10` for tighter budgets.
- `FAIL` — cost exceeded the threshold; the run exits with code 1.
- `SKIP` — benchmark missing from the baseline (won't fail; regenerate baselines to arm it).

Instructions are the better gate; memory is reported for information.

## Baselines

`benchmarks/baselines.txt` is committed. Procedure to refresh it:

1. Run `cargo run --release -p benchmarks -- --update-baseline` and inspect the
   printed report — you want to see the *expected* improvement or at least no
   unintended regression.
2. Commit the regenerated file with a message that explains *why* costs
   legitimately changed (e.g. "feat(matching-engine): ... cheaper sweep loop").
   A failing baseline update without a story is treated as a regression.
3. CI runs that touch the benchmarked paths will fail until the baseline is
   refreshed with an explanation.

## Where benchmarks run

- On demand: `.github/workflows/benchmarks.yml` (manual `Run workflow`, also
  triggered by PRs that touch the benchmarked contracts) runs the report and
  `--check`.
- Locally for developers before opening a PR against the three contracts.

## Cost model notes

- Soroban has no single "gas" number; costs are instruction- and memory-slot
  based, charged per invocation, dominated by storage entry writes. Soroban
  fees are governed by a snapshot of Pubnet fee parameters; avoid drawing
  absolute conclusions from instruction counts alone — treat them as a
  relative regression alarm.
- For a definitive view on a specific network, simulate with
  `stellar contract invoke --cost` against a testnet snapshot as well.
- See `docs/contract-sizes.md` for binary size (a separate release constraint,
  64 KiB), and `docs/architecture.md` for the overall system.
