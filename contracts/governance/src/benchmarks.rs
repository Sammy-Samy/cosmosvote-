//! Governance contract — instruction count benchmarks.
//!
//! Issue #113: instruction counts are compared against the baseline stored
//! in `tests/baselines/perf_contract_execution.json` (versioned/committed)
//! instead of hardcoded constants, so a regression shows up as a diff in
//! that file during review as well as a CI failure. Run with
//! `BENCH_UPDATE=1` (or `make bench-update`) to intentionally refresh the
//! baseline after a deliberate, reviewed change in instruction cost.

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String};
use cosmosvote_token::TokenContractClient;
use crate::{types::Vote, GovernanceContract, GovernanceContractClient};
use cosmosvote_token::TokenContract;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Baseline file handling
// ---------------------------------------------------------------------------

/// Path to the committed instruction-count baseline, resolved relative to
/// this crate's manifest dir so the test works regardless of cwd.
fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/baselines/perf_contract_execution.json")
}

fn load_baselines() -> serde_json::Value {
    let path = baseline_path();
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read baseline file {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("invalid JSON in baseline file {}: {e}", path.display()))
}

fn baseline_for(op: &str) -> u64 {
    let baselines = load_baselines();
    baselines[op]
        .as_u64()
        .unwrap_or_else(|| panic!("baseline file missing numeric field \"{op}\""))
}

/// When `BENCH_UPDATE=1` is set (via `make bench-update`), persist the
/// freshly measured instruction count as the new baseline for `op` instead
/// of asserting against the old one.
fn maybe_update_baseline(op: &str, instructions: u64) -> bool {
    if std::env::var("BENCH_UPDATE").ok().as_deref() != Some("1") {
        return false;
    }
    let path = baseline_path();
    let mut baselines = load_baselines();
    baselines[op] = serde_json::Value::from(instructions);
    fs::write(&path, serde_json::to_string_pretty(&baselines).unwrap() + "\n")
        .unwrap_or_else(|e| panic!("failed to write baseline file {}: {e}", path.display()));
    eprintln!("[bench-update] {op}: baseline updated to {instructions} instructions");
    true
}

/// Assert the measured instruction count is within 10% of the baseline for
/// `op`, unless BENCH_UPDATE=1 is set (in which case the baseline is
/// refreshed instead of asserted against).
///
/// Always emits a stable, machine-readable line `BENCH <function> <instructions>`
/// so CI can parse per-function instruction counts and compare them against
/// `wasm_size_baseline.txt`.
fn check_against_baseline(op: &str, instructions: u64) {
    // Emit the machine-readable result line first, so it is captured even if
    // the assertion below fails.
    println!("BENCH {op} {instructions}");

    if maybe_update_baseline(op, instructions) {
        return;
    }
    let baseline = baseline_for(op);
    let limit = threshold(baseline);
    assert!(
        instructions <= limit,
        "{op} used {instructions} instructions, exceeds 10% over baseline {baseline} \
         (see tests/baselines/perf_contract_execution.json; run `make bench-update` if this \
         regression is intentional)",
    );
}

// ---------------------------------------------------------------------------
// Benchmark runner
// ---------------------------------------------------------------------------

fn setup_env() -> (Env, GovernanceContractClient<'static>, TokenContractClient<'static>, Address, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    let token_id = env.register(TokenContract, ());
    let token = TokenContractClient::new(&env, &token_id);
    token.initialize(
        &admin, 
        &1_000_000_000_000i128, 
        &String::from_str(&env, "CosmosVote"), 
        &String::from_str(&env, "VOTE"), 
        &7u32
    );
    token.mint(&admin, &proposer, &10_000_000_000i128);

    let gov_id = env.register(GovernanceContract, ());
    let gov = GovernanceContractClient::new(&env, &gov_id);
    gov.initialize(&admin, &token_id, &0i128, &0u64, &0u32, &false, &None, &0u32);

    (env, gov, token, admin, proposer)
}

fn threshold(baseline: u64) -> u64 {
    baseline + (baseline / 10)
}

/// Measure the CPU instruction cost of a single contract invocation and
/// check it against the committed baseline for `op`.
fn measure<F: FnOnce()>(env: &Env, gov: &GovernanceContractClient, op: &str, f: F) {
    env.as_contract(&gov.address, || {
        env.budget().reset_unlimited();
    });
    let instructions_before = env.budget().cpu_instruction_cost();
    f();
    let instructions = env.budget().cpu_instruction_cost() - instructions_before;
    check_against_baseline(op, instructions);
}

#[test]
fn bench_create_proposal() {
    let (env, gov, _token, _admin, proposer) = setup_env();

    env.as_contract(&gov.address, || {
        env.budget().reset_unlimited();
    });
    
    let instructions_before = env.budget().cpu_instruction_cost();
    gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Vote Benchmark"),
        &String::from_str(&env, "Measuring instruction count for cast_vote"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );
    let instructions = env.budget().cpu_instruction_cost() - instructions_before;

    check_against_baseline("create_proposal", instructions);
}

#[test]
fn bench_cast_vote() {
    let (env, gov, token, admin, proposer) = setup_env();
    
    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Vote Benchmark"),
        &String::from_str(&env, "Measuring instruction count for cast_vote"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);

    env.as_contract(&gov.address, || {
        env.budget().reset_unlimited();
    });
    let instructions_before = env.budget().cpu_instruction_cost();
    gov.cast_vote(&voter, &id, &Vote::Yes);
    let instructions = env.budget().cpu_instruction_cost() - instructions_before;

    check_against_baseline("cast_vote", instructions);
}

/// Issue #471: the typical (happy-path) vote must stay at least 10% below the
/// pre-optimisation baseline recorded in `tests/baselines/perf_contract_execution.json`
/// under `cast_vote_pre_optimisation`. This guards against regressions that
/// reintroduce redundant storage reads in `cast_vote`.
///
/// The benchmark exercises the reordered fail-fast path: the cheap guards
/// (proposal existence/state and the `has_voted` flag) run before the more
/// expensive voter-balance and delegation lookups, so a typical vote performs
/// the minimum number of ledger reads.
#[test]
fn bench_cast_vote_optimised_path() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Vote Benchmark"),
        &String::from_str(&env, "Measuring instruction count for cast_vote"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);

    env.as_contract(&gov.address, || {
        env.budget().reset_unlimited();
    });
    let instructions_before = env.budget().cpu_instruction_cost();
    gov.cast_vote(&voter, &id, &Vote::Yes);
    let instructions = env.budget().cpu_instruction_cost() - instructions_before;

    // Keep the standard baseline check (10% tolerance) for the current path.
    check_against_baseline("cast_vote", instructions);

    // Additionally assert the optimised path is >= 10% cheaper than the
    // pre-optimisation baseline. Skip when refreshing baselines.
    if std::env::var("BENCH_UPDATE").ok().as_deref() == Some("1") {
        return;
    }
    let pre = baseline_for("cast_vote_pre_optimisation");
    let required = pre - (pre / 10);
    assert!(
        instructions <= required,
        "cast_vote used {instructions} instructions, expected at least 10% below the \
         pre-optimisation baseline {pre} (limit {required}); reordering/batching of storage \
         reads in cast_vote may have regressed (see tests/baselines/perf_contract_execution.json)",
    );
}

#[test]
fn bench_finalise() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Finalise Benchmark"),
        &String::from_str(&env, "Measuring instruction count for finalise"),
        &1_000i128,
        &3600u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);
    gov.cast_vote(&voter, &id, &Vote::Yes);

    let end_time = gov.get_proposal(&id).end_time;
    env.ledger().with_mut(|l| l.timestamp = end_time + 1);

    env.as_contract(&gov.address, || {
        env.budget().reset_unlimited();
    });
    let instructions_before = env.budget().cpu_instruction_cost();
    gov.finalise(&id);
    let instructions = env.budget().cpu_instruction_cost() - instructions_before;

    check_against_baseline("finalise", instructions);
}

#[test]
fn bench_retract_vote() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Retract Benchmark"),
        &String::from_str(&env, "Measuring instruction count for retract_vote"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);
    gov.cast_vote(&voter, &id, &Vote::Yes);

    measure(&env, &gov, "retract_vote", || {
        gov.retract_vote(&voter, &id);
    });
}

#[test]
fn bench_cancel() {
    let (env, gov, _token, _admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Cancel Benchmark"),
        &String::from_str(&env, "Measuring instruction count for cancel"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    measure(&env, &gov, "cancel", || {
        gov.cancel(&proposer, &id);
    });
}

#[test]
fn bench_execute() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Execute Benchmark"),
        &String::from_str(&env, "Measuring instruction count for execute"),
        &1_000i128,
        &3600u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);
    gov.cast_vote(&voter, &id, &Vote::Yes);

    let end_time = gov.get_proposal(&id).end_time;
    env.ledger().with_mut(|l| l.timestamp = end_time + 1);
    gov.finalise(&id);

    measure(&env, &gov, "execute", || {
        gov.execute(&id);
    });
}

#[test]
fn bench_get_proposal() {
    let (env, gov, _token, _admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Get Proposal Benchmark"),
        &String::from_str(&env, "Measuring instruction count for get_proposal"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    measure(&env, &gov, "get_proposal", || {
        gov.get_proposal(&id);
    });
}

#[test]
fn bench_get_proposals() {
    let (env, gov, _token, _admin, proposer) = setup_env();

    for i in 0..3u32 {
        gov.create_proposal(
            &proposer,
            &String::from_str(&env, "Get Proposals Benchmark"),
            &String::from_str(&env, "Measuring instruction count for get_proposals"),
            &1_000_000i128,
            &604_800u64,
            &None,
            &None,
        );
        let _ = i;
    }

    measure(&env, &gov, "get_proposals", || {
        gov.get_proposals(&0u32, &10u32);
    });
}

#[test]
fn bench_get_config() {
    let (env, gov, _token, _admin, _proposer) = setup_env();

    measure(&env, &gov, "get_config", || {
        gov.get_config();
    });
}

#[test]
fn bench_proposal_count() {
    let (env, gov, _token, _admin, proposer) = setup_env();

    gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Count Benchmark"),
        &String::from_str(&env, "Measuring instruction count for proposal_count"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    measure(&env, &gov, "proposal_count", || {
        gov.proposal_count();
    });
}

#[test]
fn bench_has_voted() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Has Voted Benchmark"),
        &String::from_str(&env, "Measuring instruction count for has_voted"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);
    gov.cast_vote(&voter, &id, &Vote::Yes);

    measure(&env, &gov, "has_voted", || {
        gov.has_voted(&id, &voter);
    });
}

#[test]
fn bench_get_vote() {
    let (env, gov, token, admin, proposer) = setup_env();

    let id = gov.create_proposal(
        &proposer,
        &String::from_str(&env, "Get Vote Benchmark"),
        &String::from_str(&env, "Measuring instruction count for get_vote"),
        &1_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    let voter = Address::generate(&env);
    token.mint(&admin, &voter, &1_000i128);
    gov.cast_vote(&voter, &id, &Vote::Yes);

    measure(&env, &gov, "get_vote", || {
        gov.get_vote(&id, &voter);
    });
}

#[test]
fn bench_set_admin() {
    let (env, gov, _token, admin, _proposer) = setup_env();
    let new_admin = Address::generate(&env);

    measure(&env, &gov, "set_admin", || {
        gov.set_admin(&admin, &new_admin);
    });
}

#[test]
fn bench_set_config() {
    let (env, gov, _token, admin, _proposer) = setup_env();

    measure(&env, &gov, "set_config", || {
        gov.set_config(&admin, &0i128, &0u64, &0u32, &false, &None, &0u32);
    });
}
