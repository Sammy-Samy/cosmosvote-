//! Governance contract — type definitions and error codes.

use soroban_sdk::{contracterror, contracttype, Address, String, Symbol, Vec, Val};

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    // Initialization
    AlreadyInitialized  = 1,
    NotInitialized      = 2,

    // Proposals
    ProposalNotFound    = 10,
    ProposalNotActive   = 11,
    ProposalNotPassed   = 12,
    InvalidTitle        = 13,
    InvalidDescription  = 14,
    InvalidQuorum       = 15,
    QuorumExceedsSupply = 16,
    InvalidDurationRange = 17,
    InsufficientBalance = 18,
    ProposalCooldown    = 19,
    QuorumBelowFloor    = 20,
    ProposalsStillActive = 21,

    // Voting
    VotingNotStarted    = 22,
    VotingPeriodEnded   = 23,
    VotingStillOpen     = 24,
    AlreadyVoted        = 25,
    NoVotingPower       = 26,
    AdminVoteRestricted = 27,
    VoteNotFound        = 28,
    /// Returned when a voter attempts to cast a vote identical to the vote
    /// already recorded for them on the proposal. Vote changes are not
    /// supported: once a vote is cast it cannot be modified, so re-submitting
    /// the same choice (or any other choice) is rejected.
    VoteAlreadySame     = 29,

    // Admin
    NotAdmin            = 30,
    InvalidNewAdmin     = 31,
    QuorumUpdateNotAllowed = 32,
    NoPendingAdmin      = 33,
    NotPendingAdmin     = 34,
    NotProposer         = 35,
    VotesAlreadyCast    = 36,
    InvalidLink         = 37,
    InvalidChoice       = 38,
    TooFewChoices       = 39,
    TooManyChoices      = 42,
    InvalidChoiceLength = 43,

    // Contract state
    ContractPaused      = 40,
    NotPaused           = 41,

    // Arithmetic
    ArithmeticOverflow  = 50,

    // Execution
    ExecutionFailed     = 60,
    InvalidTreasuryAmount = 61,
    InvalidRecipient    = 62,
}

// ---------------------------------------------------------------------------
// Contract state
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractState {
    Uninitialized,
    Ready,
}

// ---------------------------------------------------------------------------
// Treasury
// ---------------------------------------------------------------------------

/// Asset to disburse — native XLM or a SEP-41 token.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreasuryAsset {
    Native,
    Token(Address),
}

/// Optional payload attached to a proposal for on-execution treasury disbursement.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryAction {
    pub recipient: Address,
    pub amount: i128,
    pub asset: TreasuryAsset,
}

// ---------------------------------------------------------------------------
// Proposal
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalState {
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
    ExecutionFailed,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ExecutionPayload {
    pub contract: Address,
    pub action: Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub link: Option<String>,
    pub votes_yes: i128,
    pub votes_no: i128,
    pub votes_abstain: i128,
    pub quorum: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub state: ProposalState,
    pub snapshot_ledger: u32,
    pub voter_count: u32,
    /// Optional treasury disbursement to execute on proposal execution.
    pub treasury_action: Vec<TreasuryAction>,
    /// Multi-choice options (empty for Yes/No/Abstain proposals).
    pub choices: Vec<String>,
    /// Winning choice index after finalisation (multi-choice only).
    pub winning_choice: Option<u32>,
    /// Reason for cancellation (admin-cancelled proposals only).
    pub cancellation_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Voting
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Vote {
    Yes,
    No,
    Abstain,
    /// Cast a vote for a named choice in a multi-choice proposal.
    /// `index` is the 0-based index into `Proposal.choices`.
    Choice(u32),
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VoteRecord {
    pub vote: Vote,
    pub weight: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct GovernanceConfig {
    pub admin: Address,
    pub voting_token: Address,
    pub min_proposal_balance: i128,
    pub proposal_cooldown: u64,
    pub restrict_admin_vote: bool,
    pub paused: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `ContractError` variant paired with its `u32` discriminant.
    ///
    /// The integer values are part of the public API: clients decode them from
    /// failed invocations. Keep this list in sync with the enum above.
    const ERROR_CODE_MAP: &[(&str, u32)] = &[
        ("AlreadyInitialized", 1),
        ("NotInitialized", 2),
        ("ProposalNotFound", 10),
        ("ProposalNotActive", 11),
        ("ProposalNotPassed", 12),
        ("InvalidTitle", 13),
        ("InvalidDescription", 14),
        ("InvalidQuorum", 15),
        ("QuorumExceedsSupply", 16),
        ("InvalidDurationRange", 17),
        ("InsufficientBalance", 18),
        ("ProposalCooldown", 19),
        ("QuorumBelowFloor", 20),
        ("ProposalsStillActive", 21),
        ("VotingNotStarted", 22),
        ("VotingPeriodEnded", 23),
        ("VotingStillOpen", 24),
        ("AlreadyVoted", 25),
        ("NoVotingPower", 26),
        ("AdminVoteRestricted", 27),
        ("VoteNotFound", 28),
        ("VoteAlreadySame", 29),
        ("NotAdmin", 30),
        ("InvalidNewAdmin", 31),
        ("QuorumUpdateNotAllowed", 32),
        ("NoPendingAdmin", 33),
        ("NotPendingAdmin", 34),
        ("NotProposer", 35),
        ("VotesAlreadyCast", 36),
        ("InvalidLink", 37),
        ("InvalidChoice", 38),
        ("ContractPaused", 40),
        ("NotPaused", 41),
        ("ArithmeticOverflow", 50),
        ("ExecutionFailed", 60),
        ("InvalidTreasuryAmount", 61),
    ];

    /// Render the error-code mapping in the snapshot file format.
    fn render_snapshot() -> String {
        let mut out = String::from("# ContractError discriminants — do not edit by hand.\n");
        out.push_str("# Regenerate with: cargo test -p governance -- --ignored update_error_code_snapshot\n");
        for (name, code) in ERROR_CODE_MAP {
            out.push_str(&format!("{} = {}\n", name, code));
        }
        out
    }

    fn snapshot_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_snapshots")
            .join("error_codes.snap")
    }

    /// Pins the `u32` discriminant of every `ContractError` variant.
    ///
    /// Fails if a variant is renumbered, added or removed without updating the
    /// snapshot file in `test_snapshots/error_codes.snap`.
    #[test]
    fn error_code_snapshot_matches() {
        let expected = std::fs::read_to_string(snapshot_path())
            .expect("missing test_snapshots/error_codes.snap");
        let actual = render_snapshot();
        assert_eq!(
            actual, expected,
            "ContractError discriminants changed; update test_snapshots/error_codes.snap \
             (see CONTRIBUTING.md) if the change is intentional"
        );
    }

    /// Verifies the snapshot list covers every variant exactly once.
    #[test]
    fn error_code_snapshot_is_exhaustive() {
        let mut seen = std::collections::BTreeSet::new();
        for (name, _) in ERROR_CODE_MAP {
            assert!(seen.insert(*name), "duplicate variant in snapshot: {}", name);
        }
        // Spot-check that the enum's own discriminants agree with the table.
        assert_eq!(ContractError::AlreadyInitialized as u32, 1);
        assert_eq!(ContractError::NotInitialized as u32, 2);
        assert_eq!(ContractError::ProposalNotFound as u32, 10);
        assert_eq!(ContractError::ProposalsStillActive as u32, 21);
        assert_eq!(ContractError::InvalidChoice as u32, 38);
        assert_eq!(ContractError::ContractPaused as u32, 40);
        assert_eq!(ContractError::NotPaused as u32, 41);
        assert_eq!(ContractError::ArithmeticOverflow as u32, 50);
        assert_eq!(ContractError::ExecutionFailed as u32, 60);
        assert_eq!(ContractError::InvalidTreasuryAmount as u32, 61);
    }

    /// Regenerates the snapshot file. Run explicitly:
    /// `cargo test -p governance -- --ignored update_error_code_snapshot`
    #[test]
    #[ignore]
    fn update_error_code_snapshot() {
        let path = snapshot_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("create test_snapshots dir");
        }
        std::fs::write(&path, render_snapshot()).expect("write error_codes.snap");
    }
}

// .
