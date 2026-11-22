# Mutation Testing

CosmosVote uses [cargo-mutants](https://mutants.rs/) to verify that the test suite
actually catches bugs, not just executes code paths.

## What is mutation testing?

The tool makes small, targeted changes to the source code (mutants) — flipping a `>` to
`>=`, removing a `return Err(...)`, etc. — and checks whether the existing tests fail.
A mutant that is **not caught** (a "surviving mutant") indicates a gap in test coverage.

## Running locally

```bash
# Install
cargo install cargo-mutants --locked

# Run against the governance contract
cargo mutants \
  --package cosmosvote-governance \
  --features testutils \
  --output mutants-out \
  -- --features testutils

# View results
cat mutants-out/missed.txt   # surviving mutants (test gaps)
cat mutants-out/caught.txt   # caught mutants (good coverage)
```

## CI integration

Mutation testing runs automatically on every push to `main` and on PRs that touch
`contracts/governance/src/lib.rs` or `contracts/governance/src/test.rs`.

Results are uploaded as a CI artifact (`mutants-report`) for 14 days.

## Mutation score threshold

CI **enforces** a minimum mutation score. The threshold is defined in
`.github/workflows/mutation-testing.yml` via the `MUTATION_SCORE_THRESHOLD`
environment variable (currently **70%**).

- The mutation score is computed as
  `caught / (caught + missed) * 100`, where `caught` is the number of mutants
  killed by the test suite and `missed` is the number of surviving mutants.
  Mutants reported as `unviable` or `timeout` are excluded from the denominator.
- After `cargo mutants` finishes, the workflow parses the results, writes the
  computed score to `mutants-out/mutation-score.txt`, and compares it against
  the threshold.
- **If the score is below the threshold, the CI step fails** and the PR cannot be
  merged until the score is restored (by adding tests that kill surviving mutants
  or by removing dead code that generates unkillable mutants).
- The score file and the full `mutants-out` directory are uploaded in the
  `mutants-report` artifact, so the score is tracked over time across CI runs.

## Baseline mutation score

| Metric | Value |
|--------|-------|
| Tool | cargo-mutants |
| Target | `cosmosvote-governance` core logic |
| Threshold (enforced in CI) | 70% |
| Score computation | `caught / (caught + missed) * 100` |
| Score location | `mutants-report` artifact → `mutants-out/mutation-score.txt` |
| Baseline run | Pending first CI execution |
| Surviving mutants | To be documented after first run |

> Update this table after each CI run by reviewing the `mutants-report` artifact,
> and record the score history below to track the trend over time.

### Score history

| Date | Commit | Mutation score | Threshold | Result |
|------|--------|----------------|-----------|--------|
| _Pending first CI execution_ | — | — | 70% | — |

## Addressing surviving mutants

For each surviving mutant in `mutants-out/missed.txt`:

1. Understand what the mutant changes (the diff is shown inline).
2. Add a targeted test that would catch that change.
3. Re-run `cargo mutants` to confirm the mutant is now caught.

Common patterns that produce surviving mutants:

- Off-by-one boundary conditions (e.g., `>` vs `>=` in time checks)
- Error paths that are reachable but not asserted in tests
- Arithmetic edge cases (zero, max values)
