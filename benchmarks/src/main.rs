//! Gas/resource-cost benchmarks for the ICTP contracts (issue #53).
//!
//! Reports a readable table of CPU instructions and memory bytes for the
//! critical paths, measured with Soroban's cost-estimate budget (available
//! after every top-level contract invocation).
//!
//! Usage:
//!   cargo run --release -p benchmarks                    # print the table
//!   cargo run --release -p benchmarks -- --check         # exit 1 on regressions (vs baselines.txt)
//!   cargo run --release -p benchmarks -- --update-baseline
//!
//! `--check` compares each benchmark's instruction count against the
//! committed `benchmarks/baselines.txt`. A benchmark passes when its cost
//! grows by at most `REGRESSION_THRESHOLD` (default 1.25x). Override with
//! the `BENCHMARK_REGRESSION_THRESHOLD` environment variable.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process;

use governance::{Governance, GovernanceClient};
use matching_engine::{MatchingEngine, MatchingEngineClient, OrderSide};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use trade_settlement::{TradeSettlement, TradeSettlementClient};

const REGRESSION_THRESHOLD: f64 = 1.25;
const BASELINE_PATH: &str = "benchmarks/baselines.txt";

/// One measured top-level contract invocation.
struct Row {
    name: &'static str,
    instructions: u64,
    memory: u64,
    note: String,
}

fn measured_row(env: &Env, name: &'static str, note: impl Into<String>) -> Row {
    let budget = env.cost_estimate().budget();
    Row {
        name,
        instructions: budget.cpu_instruction_cost(),
        memory: budget.memory_bytes_cost(),
        note: note.into(),
    }
}

/* ------------------------------------------------------------------ */
/* Fixture helpers                                                     */
/* ------------------------------------------------------------------ */

struct TokenPair {
    env: Env,
    base: Address,
    quote: Address,
    alice: Address,
    bob: Address,
}

/// Deploys two stellar-asset-contracts and funds both parties symmetrically.
fn funded_tokens() -> TokenPair {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let base_sac = env.register_stellar_asset_contract_v2(admin.clone());
    let base = base_sac.address();
    let base_admin = token::StellarAssetClient::new(&env, &base);
    base_admin.mint(&alice, &100_000);
    base_admin.mint(&bob, &100_000);

    let quote_sac = env.register_stellar_asset_contract_v2(admin);
    let quote = quote_sac.address();
    let quote_admin = token::StellarAssetClient::new(&env, &quote);
    quote_admin.mint(&alice, &100_000);
    quote_admin.mint(&bob, &100_000);

    TokenPair {
        env,
        base,
        quote,
        alice,
        bob,
    }
}

/* ------------------------------------------------------------------ */
/* Matching-engine benchmarks                                          */
/* ------------------------------------------------------------------ */

fn setup_order_book(t: &TokenPair) -> MatchingEngineClient<'static> {
    let contract_id = t.env.register(MatchingEngine, ());
    let client = MatchingEngineClient::new(&t.env, &contract_id);
    let admin = Address::generate(&t.env);
    client.initialize(&admin);
    client
}

/// Seeds `count` resting sells with 4 price levels (100, 110, 120, 130).
fn seed_asks(t: &TokenPair, client: &MatchingEngineClient, count: u32) {
    let seller = Address::generate(&t.env);
    for i in 0..count {
        let price: i128 = 100 + ((i % 4) as i128) * 10;
        client.place_order(&seller, &t.base, &t.quote, &OrderSide::Sell, &price, &1);
    }
}

fn bench_place_order_warm_book() -> Row {
    let t = funded_tokens();
    let client = setup_order_book(&t);
    seed_asks(&t, &client, 32);

    let buyer = Address::generate(&t.env);
    client.place_order(&buyer, &t.base, &t.quote, &OrderSide::Buy, &100, &8);

    measured_row(
        &t.env,
        "matching-engine/place_order-warm-book(32-asks)",
        "insert buy above the book",
    )
}

fn bench_match_orders_sweep() -> Row {
    let t = funded_tokens();
    let client = setup_order_book(&t);

    // 32 resting sells at rising prices.
    seed_asks(&t, &client, 32);

    // An aggressive buyer sweeping the entire book.
    let buyer = Address::generate(&t.env);
    client.place_order(&buyer, &t.base, &t.quote, &OrderSide::Buy, &250, &100);

    let trades = client.match_orders(&t.base, &t.quote);
    measured_row(
        &t.env,
        "matching-engine/match_orders-sweep(32-asks)",
        format!("matched {} trades", trades.len()),
    )
}

/* ------------------------------------------------------------------ */
/* Trade-settlement benchmarks                                         */
/* ------------------------------------------------------------------ */

fn bench_settle_trade() -> Row {
    let t = funded_tokens();
    let contract_id = t.env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&t.env, &contract_id);
    t.env.ledger().set_timestamp(1_000);

    let id = client.open(&t.alice, &t.bob, &t.base, &t.quote, &10, &15);
    let _ = id;

    let trade = client.settle_trade(&id, &t.alice);
    measured_row(
        &t.env,
        "trade-settlement/settle_trade",
        format!("status={:?}", trade.status),
    )
}

/// Opens 8 trades between Alice and Bob alternating direction and calls
/// the netted batch settlement.
fn bench_settle_batch_netted(count: u32) -> Row {
    let t = funded_tokens();
    let contract_id = t.env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&t.env, &contract_id);
    t.env.ledger().set_timestamp(1_000);

    let mut ids: soroban_sdk::Vec<u64> = soroban_sdk::Vec::new(&t.env);
    for i in 0..count {
        // Alternate direction A→B and B→A so netting cancels most transfers.
        let (buyer, seller) = if i % 2 == 0 {
            (t.alice.clone(), t.bob.clone())
        } else {
            (t.bob.clone(), t.alice.clone())
        };
        let id = client.open(
            &buyer,
            &seller,
            &t.base,
            &t.quote,
            &(10 + i as i128),
            &(15 + i as i128),
        );
        ids.push_back(id);
        t.env.ledger().set_timestamp(1_000 + i as u64 + 1);
    }

    let result = client.settle_batch_netted(&ids, &t.alice);
    measured_row(
        &t.env,
        "trade-settlement/settle_batch_netted-8",
        format!(
            "net transfers {} of {} original",
            result.net_transfers_executed, result.original_transfers_count
        ),
    )
}

/* ------------------------------------------------------------------ */
/* Governance benchmarks                                               */
/* ------------------------------------------------------------------ */

/// Opens a proposal and returns its id along with a contract client and env.
fn governance_fixture() -> (Env, GovernanceClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let contract_id = env.register(Governance, ());
    let client = GovernanceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn open_proposal(env: &Env, client: &GovernanceClient) -> u64 {
    let proposer = Address::generate(env);
    client.propose(
        &proposer,
        &symbol_short!("feestoreq"),
        &symbol_short!("fee"),
        &100,     // value
        &10,      // start_time
        &20,      // end_time
        &5,       // timelock
        &100u128, // quorum
        &150u128, // threshold
    )
}

fn bench_governance_propose() -> Row {
    let (env, client, _admin) = governance_fixture();

    env.ledger().set_timestamp(1);
    let proposal_id = open_proposal(&env, &client);

    measured_row(
        &env,
        "governance/propose",
        format!("proposal_id={}", proposal_id),
    )
}

fn bench_governance_finalize(voter_count: u32) -> Row {
    let (env, client, _admin) = governance_fixture();

    let proposal_id = open_proposal(&env, &client);
    env.ledger().set_timestamp(15);
    for _ in 0..voter_count {
        let voter = Address::generate(&env);
        client.cast_vote(&proposal_id, &voter, &true, &30u128);
    }

    env.ledger().set_timestamp(25);
    client.finalize(&proposal_id);

    measured_row(
        &env,
        match voter_count {
            9 => "governance/finalize_9_votes",
            39 => "governance/finalize_39_votes",
            _ => "governance/finalize_other_votes",
        },
        format!("votes={}", voter_count),
    )
}

fn bench_governance_execute_proposal() -> Row {
    let (env, client, _admin) = governance_fixture();

    let proposal_id = open_proposal(&env, &client);
    env.ledger().set_timestamp(15);
    let voter = Address::generate(&env);
    client.cast_vote(&proposal_id, &voter, &true, &200u128);
    env.ledger().set_timestamp(25);
    client.finalize(&proposal_id);
    client.queue_execution(&proposal_id, &voter);
    env.ledger().set_timestamp(30);

    client.execute_proposal(&proposal_id, &voter);

    measured_row(
        &env,
        "governance/execute_proposal",
        format!("proposal_id={}", proposal_id),
    )
}

/* ------------------------------------------------------------------ */
/* Runner                                                              */
/* ------------------------------------------------------------------ */

fn run_all() -> Vec<Row> {
    vec![
        bench_place_order_warm_book(),
        bench_match_orders_sweep(),
        bench_settle_trade(),
        bench_settle_batch_netted(8),
        bench_governance_propose(),
        bench_governance_finalize(9),
        bench_governance_finalize(39),
        bench_governance_execute_proposal(),
    ]
}

fn load_baseline(path: &str) -> BTreeMap<String, u64> {
    let mut map = BTreeMap::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                if let Ok(v) = value.parse::<u64>() {
                    map.insert(name.to_string(), v);
                }
            }
        }
    }
    map
}

fn baseline_path() -> String {
    env::current_dir()
        .map(|d| d.join(BASELINE_PATH).to_string_lossy().into_owned())
        .unwrap_or_else(|_| BASELINE_PATH.to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode_update = args.iter().any(|a| a == "--update-baseline");
    let mode_check = args.iter().any(|a| a == "--check");

    let threshold = env::var("BENCHMARK_REGRESSION_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| *v > 1.0)
        .unwrap_or(REGRESSION_THRESHOLD);

    let rows = run_all();

    println!("ICTP benchmark report (Soroban cost-estimate budget)");
    println!();
    println!(
        "{:<42} {:>13} {:>14}",
        "benchmark", "instructions", "memory_bytes"
    );
    println!("{:-<42} {:-<13} {:-<14}", "-", "-", "-");
    for row in &rows {
        println!(
            "{:<42} {:>13} {:>14}",
            row.name, row.instructions, row.memory
        );
        println!("    {:<40}", row.note);
    }

    if mode_update {
        let mut body = String::from(
            "# Baseline CPU instructions per benchmark (regenerate: cargo run --release -p benchmarks -- --update-baseline)\n",
        );
        for row in &rows {
            body.push_str(&format!("{} {}\n", row.name, row.instructions));
        }
        fs::write(baseline_path(), body).expect("failed to write baselines.txt");
        println!("\nWrote baseline to {}", BASELINE_PATH);
        return;
    }

    if !mode_check {
        return;
    }

    let baseline = load_baseline(BASELINE_PATH);
    if baseline.is_empty() {
        println!(
            "\nNo baseline found at {} — record one with --update-baseline (not failing).",
            BASELINE_PATH
        );
        return;
    }

    println!();
    println!("Baseline comparison (threshold = x{:.2})", threshold);
    let mut failed = false;
    for row in &rows {
        match baseline.get(row.name) {
            None => {
                println!("SKIP  {:<42} no baseline entry", row.name);
            }
            Some(&base) if base == 0 => {
                println!("SKIP  {:<42} baseline entry is zero", row.name);
            }
            Some(&base) => {
                let ratio = row.instructions as f64 / base as f64;
                let ok = ratio <= threshold;
                if !ok {
                    failed = true;
                }
                println!(
                    "{}  {:<42} {:.2}x ({} vs baseline {})",
                    if ok { "PASS" } else { "FAIL" },
                    row.name,
                    ratio,
                    row.instructions,
                    base
                );
            }
        }
    }

    if failed {
        process::exit(1);
    }
}
