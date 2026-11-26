# Gas & Storage Benchmarks

This document records Soroban CPU instruction and memory costs for the core governance operations: proposal creation, voting, finalization, cancellation, and execution.

## Running the Benchmarks

Benchmarks are executed against the **`wasm32-unknown-unknown`** target, not the native host. Native runs use the host's 64-bit instruction set and allocator, which produces instruction counts that do not correspond to what the Soroban VM actually charges on-chain. Only WASM-target numbers are meaningful for fee estimation and regression tracking.

```bash
# Build the contract for the WASM target first
cargo build --target wasm32-unknown-unknown --release -p governance

# Run the benchmark suite against the WASM artifact
cargo test bench_ --features testutils --target wasm32-unknown-unknown -- --nocapture
```

Each test prints the CPU instruction count and memory bytes for a single isolated call.

## Benchmark Results

> **Environment:** Soroban SDK (testutils environment, `wasm32-unknown-unknown` target, no network overhead)  
> Results reflect in-VM cost only. Network fees depend on ledger congestion.

| Operation | CPU Instructions | Memory (bytes) | Notes |
|-----------|-----------------|----------------|-------|
| `create_proposal` | 1,284,512 | 412,880 | Writes Proposal to persistent storage, increments counter |
| `cast_vote` | 1,731,904 | 587,216 | Reads token balance + proposal, writes HasVoted + VoteRecord |
| `finalise` (passed) | 968,320 | 301,472 | Reads proposal + vote totals, writes new state |
| `finalise` (rejected) | 942,784 | 294,608 | Same path, quorum not met |
| `cancel` | 612,096 | 188,336 | Admin auth + state write |
| `execute` | 655,360 | 201,904 | Admin auth + state write |

Numbers above are the recorded baseline for the current release. Re-run the suite after any change to the governance contract and update this table together with `wasm_size_baseline.txt`.

---

## Baseline File & Regression Gate

Per-function instruction counts are tracked in [`wasm_size_baseline.txt`](../wasm_size_baseline.txt) at the repository root. The file records the compiled WASM size and one line per public function:

```
# wasm_size_baseline.txt
wasm_size_bytes=48213

# Per-function CPU instruction baselines (wasm32-unknown-unknown)
cpu_insns.create_proposal=1284512
cpu_insns.cast_vote=1731904
cpu_insns.finalise_passed=968320
cpu_insns.finalise_rejected=942784
cpu_insns.cancel=612096
cpu_insns.execute=655360
```

CI parses this file and compares it against the freshly measured values from the WASM benchmark run. **The build fails if any function's CPU instruction count exceeds its baseline by more than 10%**, or if the compiled WASM size grows by more than 10%. This catches gas regressions introduced by code changes before they reach a release.

When a change legitimately increases cost (e.g. a new feature or a security fix), update the baseline file in the same pull request and explain the increase in the PR description. Baseline updates without justification are rejected in review.

---

## Storage Budget Summary

### Governance Contract

| Item | Storage Tier | Estimated Entries | Notes |
|------|-------------|------------------|-------|
| Admin config (`Admin`, `VotingToken`, etc.) | Instance | 6 keys | Set once at init, low cost |
| `Proposal(id)` | Persistent | 1 per proposal | ~200–400 bytes per struct |
| `HasVoted(id, voter)` | Persistent | 1 per (proposal, voter) | 1 bool — minimal |
| `VoteRecord(id, voter)` | Persistent | 1 per (proposal, voter) | ~50 bytes |
| `LastProposal(proposer)` | Persistent | 1 per proposer | 8-byte timestamp |

### Token Contract

| Item | Storage Tier | Notes |
|------|-------------|-------|
| `Balance(address)` | Persistent | ~50 bytes per holder |
| `Admin` | Instance | Single key |
| `TotalSupply` | Instance | Single i128 |
| `Allowance(owner, spender)` | Temporary | Expires each ledger |

### Storage Costs (Soroban fee schedule, approximate)

| Tier | Write fee | Read fee | TTL |
|------|-----------|----------|-----|
| Instance | Lowest | Lowest | Contract lifetime |
| Persistent | Medium | Low | Requires bump to stay live |
| Temporary | Low | Low | Expires after a few ledgers |

> **Important:** Persistent storage entries must be bumped regularly (via `env.storage().persistent().bump()`) or they expire. The governance contract bumps proposal entries on every read. If a proposal sits unread for an extended period, it may need a manual TTL bump.

---

## Interpreting Results

**CPU instructions** map to the Soroban fee resource `cpu_insns`. The network charges a fee proportional to instructions used. Higher instruction counts raise the transaction fee.

**Memory bytes** map to the `mem_bytes` resource. Large memory use increases fees and can hit the per-transaction memory cap.

Both resources are bounded per transaction by the Soroban network limits. As of Stellar Protocol 21:

| Resource | Per-transaction limit |
|----------|----------------------|
| CPU instructions | 100,000,000 |
| Memory bytes | 40,000,000 |
| Read ledger entries | 40 |
| Write ledger entries | 25 |
| Read bytes | 200,000 |
| Write bytes | 66,560 |

All CosmosVote governance operations stay well within these limits for typical usage.

---

## Benchmark Test Location

Source: [`contracts/governance/src/benchmarks.rs`](../contracts/governance/src/benchmarks.rs)

```
cargo test bench_create_proposal --features testutils -- --nocapture
cargo test bench_cast_vote       --features testutils -- --nocapture
cargo test bench_finalise_passed --features testutils -- --nocapture
cargo test bench_finalise_rejected --features testutils -- --nocapture
cargo test bench_cancel          --features testutils -- --nocapture
cargo test bench_execute         --features testutils -- --nocapture
```
