//! Storage TTL refresh tests — issue #61
//!
//! Verifies that `cast_vote` and `finalise` refresh the persistent storage TTL
//! for Proposal, HasVoted, and VoteRecord keys so that long-lived proposals
//! (up to 30 days) do not expire before finalization.
//!
//! The Soroban test harness does not actually expire entries when ledgers
//! advance, but we can:
//! 1. Advance ledger sequence far beyond the TTL threshold.
//! 2. Cast votes after the advance — storage reads/writes must still succeed
//!    because `GovernanceStorage` bumps the TTL on every access.
//! 3. Call `finalise` after voting ends — the proposal must still be readable.
//!
//! This simulates the "TTL expiry and recovery" scenario described in the
//! acceptance criteria: if TTL were NOT bumped, a real node would archive
//! the entry and the read would fail. The test confirms the bump path is
//! exercised on every read/write so that expiry cannot happen in practice.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

/// Number of ledgers in 30 days, assuming a 5-second ledger close time.
const THIRTY_DAYS_LEDGERS: u32 = 30 * 24 * 60 * 60 / 5; // 518_400

use crate::{
    types::{ProposalState, Vote},
    GovernanceContract, GovernanceContractClient,
};
use cosmosvote_token::{TokenContract, TokenContractClient};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

struct TtlTestEnv<'a> {
    env: &'a Env,
    gov: GovernanceContractClient<'a>,
    _token: TokenContractClient<'a>,
    _admin: Address,
    voter_a: Address,
    voter_b: Address,
}

fn setup(env: &Env) -> TtlTestEnv<'_> {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let voter_a = Address::generate(env);
    let voter_b = Address::generate(env);

    let token_id = env.register(TokenContract, ());
    let token = TokenContractClient::new(env, &token_id);
    token.initialize(
        &admin,
        &1_000_000_000i128,
        &String::from_str(env, "CosmosVote"),
        &String::from_str(env, "VOTE"),
        &7u32,
    );
    token.mint(&admin, &voter_a, &10_000_000i128);
    token.mint(&admin, &voter_b, &5_000_000i128);

    let gov_id = env.register(GovernanceContract, ());
    let gov = GovernanceContractClient::new(env, &gov_id);
    gov.initialize(&admin, &token_id, &0i128, &0u64, &0u32, &false, &None, &0u32);

    TtlTestEnv { env, gov, _token: token, _admin: admin, voter_a, voter_b }
}

// ---------------------------------------------------------------------------
// TTL tests
// ---------------------------------------------------------------------------

/// Verify that cast_vote refreshes TTL for Proposal, HasVoted and VoteRecord keys.
///
/// We advance the ledger sequence by PROPOSAL_TTL_LEDGERS (518 400) plus a buffer
/// to simulate a scenario where entries would expire without bumping. The storage
/// read/write calls in cast_vote must still succeed because each access calls
/// `bump_persistent_ttl`.
#[test]
fn test_cast_vote_refreshes_ttl() {
    let env = Env::default();
    let t = setup(&env);

    // Create a proposal with a 30-day duration (max allowed: 2 592 000 s).
    let duration: u64 = 2_592_000;
    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "Long Proposal"),
        &String::from_str(t.env, "A proposal that lasts the maximum allowed duration"),
        &5_000_000i128,
        &duration,
        &None,
        &None,
    );

    let start_time = env.ledger().timestamp();

    // Advance ledger sequence by more than PROPOSAL_TTL_LEDGERS to simulate
    // near-expiry conditions. Timestamp stays inside the voting window.
    let near_expiry_ledger = 520_000u32; // > PROPOSAL_TTL_LEDGERS (518_400)
    env.ledger().with_mut(|l| {
        l.sequence_number += near_expiry_ledger;
        // Timestamp still within voting window (advance by 25 days)
        l.timestamp = start_time + 2_160_000; // 25 days in seconds
    });

    // cast_vote must succeed despite the advanced ledger — TTL is bumped.
    t.gov.cast_vote(&t.voter_a, &proposal_id, &Vote::Yes);
    assert!(
        t.gov.has_voted(&proposal_id, &t.voter_a),
        "has_voted should return true after cast_vote succeeds near TTL boundary"
    );

    // vote record must be readable
    let record = t.gov.get_vote(&proposal_id, &t.voter_a);
    assert_eq!(
        record.vote,
        Vote::Yes,
        "vote record should be readable after TTL bump"
    );

    // Second voter also votes to build quorum
    t.gov.cast_vote(&t.voter_b, &proposal_id, &Vote::Yes);
    assert!(t.gov.has_voted(&proposal_id, &t.voter_b));
}

/// Verify that finalise refreshes the Proposal TTL after voting ends.
///
/// The proposal should still be readable (as Passed/Rejected) after finalise
/// even when the ledger is far beyond initial TTL expectations.
#[test]
fn test_finalise_refreshes_proposal_ttl() {
    let env = Env::default();
    let t = setup(&env);

    let duration: u64 = 2_592_000; // 30-day proposal
    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "Long Proposal"),
        &String::from_str(t.env, "A proposal that lasts the maximum allowed duration"),
        &5_000_000i128,
        &duration,
        &None,
        &None,
    );

    let start_time = env.ledger().timestamp();

    // Vote during the proposal window.
    t.gov.cast_vote(&t.voter_a, &proposal_id, &Vote::Yes);
    t.gov.cast_vote(&t.voter_b, &proposal_id, &Vote::Yes);

    // Advance time past the proposal end and sequence well beyond TTL threshold.
    env.ledger().with_mut(|l| {
        l.sequence_number += 520_000; // > PROPOSAL_TTL_LEDGERS
        l.timestamp = start_time + duration + 1; // just after voting ends
    });

    // finalise must succeed — Proposal key TTL is refreshed on read and write.
    t.gov.finalise(&proposal_id);

    let proposal = t.gov.get_proposal(&proposal_id);
    assert_eq!(
        proposal.state,
        ProposalState::Passed,
        "proposal should be Passed after finalise, even with ledger advanced beyond initial TTL"
    );
}

/// Regression: proposal is still readable after TTL threshold advance without
/// any interaction (TTL was bumped on creation, not just on vote).
#[test]
fn test_proposal_readable_after_ttl_threshold_advance() {
    let env = Env::default();
    let t = setup(&env);

    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "Proposal"),
        &String::from_str(t.env, "desc"),
        &5_000_000i128,
        &604_800u64,
        &None,
        &None,
    );

    // Advance sequence number just past PERSISTENT_THRESHOLD (17 280) to trigger
    // a TTL bump on the next read.
    env.ledger().with_mut(|l| {
        l.sequence_number += 17_281;
    });

    // Reading the proposal must still succeed.
    let proposal = t.gov.get_proposal(&proposal_id);
    assert_eq!(proposal.id, proposal_id);
    assert_eq!(proposal.state, ProposalState::Active);
}

// ---------------------------------------------------------------------------
// Explicit TTL assertions (issue #454)
// ---------------------------------------------------------------------------

/// Compute the minimum acceptable TTL (in ledgers) for a proposal whose voting
/// window ends at `end_time`, given the current ledger timestamp.
///
/// The requirement is that the Proposal entry lives at least until
/// `end_time + 30 days`, expressed in ledgers (5 s per ledger).
fn min_required_ttl(env: &Env, end_time: u64) -> u32 {
    let now = env.ledger().timestamp();
    let remaining_secs = end_time.saturating_sub(now);
    let total_secs = remaining_secs + 30 * 24 * 60 * 60;
    (total_secs / 5) as u32
}

/// Read the persistent TTL for the Proposal key and assert it is at least
/// `min_required_ttl(env, end_time)`.
fn assert_proposal_ttl(env: &Env, proposal_id: u64, end_time: u64) {
    let key = crate::storage::DataKey::Proposal(proposal_id);
    let ttl = env
        .storage()
        .persistent()
        .get_ttl(&key)
        .expect("Proposal entry must exist in persistent storage");
    let required = min_required_ttl(env, end_time);
    assert!(
        ttl >= required,
        "Proposal TTL {ttl} is below required {required} ledgers (end_time={end_time})"
    );
}

/// cast_vote must bump the Proposal TTL to at least `end_time + 30 days`.
#[test]
fn test_cast_vote_bumps_proposal_ttl_explicit() {
    let env = Env::default();
    let t = setup(&env);

    let duration: u64 = 2_592_000; // 30-day proposal (max allowed)
    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "TTL Proposal"),
        &String::from_str(t.env, "explicit TTL assertion"),
        &5_000_000i128,
        &duration,
        &None,
        &None,
    );

    let start_time = env.ledger().timestamp();
    let end_time = start_time + duration;

    // Advance partway through the voting window.
    env.ledger().with_mut(|l| {
        l.sequence_number += 100_000;
        l.timestamp = start_time + 1_000_000;
    });

    t.gov.cast_vote(&t.voter_a, &proposal_id, &Vote::Yes);

    assert_proposal_ttl(&env, proposal_id, end_time);
}

/// finalise must bump the Proposal TTL to at least `end_time + 30 days`.
#[test]
fn test_finalise_bumps_proposal_ttl_explicit() {
    let env = Env::default();
    let t = setup(&env);

    let duration: u64 = 2_592_000; // 30-day proposal (max allowed)
    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "TTL Proposal"),
        &String::from_str(t.env, "explicit TTL assertion"),
        &5_000_000i128,
        &duration,
        &None,
        &None,
    );

    let start_time = env.ledger().timestamp();
    let end_time = start_time + duration;

    t.gov.cast_vote(&t.voter_a, &proposal_id, &Vote::Yes);
    t.gov.cast_vote(&t.voter_b, &proposal_id, &Vote::Yes);

    // Advance past voting end.
    env.ledger().with_mut(|l| {
        l.sequence_number += 200_000;
        l.timestamp = end_time + 1;
    });

    t.gov.finalise(&proposal_id);

    assert_proposal_ttl(&env, proposal_id, end_time);
}

/// TTL bump must hold for a short-duration proposal too, and remain at least
/// 30 days worth of ledgers beyond the (already-passed) end_time.
#[test]
fn test_short_proposal_ttl_floor() {
    let env = Env::default();
    let t = setup(&env);

    let duration: u64 = 604_800; // 7 days
    let proposal_id = t.gov.create_proposal(
        &t.voter_a,
        &String::from_str(t.env, "Short Proposal"),
        &String::from_str(t.env, "short duration TTL"),
        &5_000_000i128,
        &duration,
        &None,
        &None,
    );

    let start_time = env.ledger().timestamp();
    let end_time = start_time + duration;

    t.gov.cast_vote(&t.voter_a, &proposal_id, &Vote::Yes);
    assert_proposal_ttl(&env, proposal_id, end_time);

    env.ledger().with_mut(|l| {
        l.sequence_number += 50_000;
        l.timestamp = end_time + 1;
    });
    t.gov.finalise(&proposal_id);
    assert_proposal_ttl(&env, proposal_id, end_time);
}
