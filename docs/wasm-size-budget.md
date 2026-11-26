# WASM Size Budget

CosmosVote CI enforces a maximum binary size for each compiled contract.

## Budget

| Contract | Budget |
|----------|--------|
| `cosmosvote_governance.wasm` | 100 KB (102 400 bytes) |
| `cosmosvote_token.wasm` | 100 KB (102 400 bytes) |

## Rationale

- **Soroban upload limit** — Soroban enforces a hard limit on WASM upload size. Staying at 100 KB leaves substantial headroom below that ceiling.
- **Transaction fees** — Larger WASMs increase upload fees. Keeping binaries small benefits deployers on testnet and mainnet.
- **Regression detection** — A CI gate catches accidental size increases from added dependencies or generated code before they reach `main`.

## CI enforcement

The `Enforce WASM size budget` step in `.github/workflows/ci.yml` (inside the `build` job) iterates over every `cosmosvote_*.wasm` artifact and fails if any exceeds the budget.

## Size regression gate

In addition to the absolute budget, CI enforces a **regression gate** against a recorded baseline. This gate is a **mandatory check** — a PR cannot merge if it fails.

- The baseline is stored in `wasm_size_baseline.txt` at the repository root.
- The `Enforce WASM size regression gate` step in `.github/workflows/ci.yml` (inside the `build` job) reads the baseline and compares each compiled `cosmosvote_*.wasm` artifact against it.
- If any binary exceeds its baseline by **more than 5%**, the step fails and the PR is blocked.

This catches gradual size creep that stays under the absolute budget but still represents an unintended increase.

### Updating the baseline

If a size increase is intentional and justified (e.g. a necessary new feature or dependency):

1. Update `wasm_size_baseline.txt` with the new sizes in the same PR that introduces the change.
2. Document the rationale in the PR description — explain why the growth is necessary and what was done to minimize it.
3. **Explicit PR approval is required.** A baseline update must be reviewed and approved by a maintainer; it is not merged automatically. Reviewers should confirm the increase is justified and that the absolute budget is still respected.

## Reducing binary size

If a contract exceeds the budget or triggers the regression gate:

1. Run `wasm-opt -Os` (from [binaryen](https://github.com/WebAssembly/binaryen)) on the artifact:
   ```bash
   wasm-opt -Os target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm \
     -o target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm
   ```
2. Audit new dependencies — prefer `no_std`-compatible, minimal crates.
3. Remove unused features from `Cargo.toml` dependency declarations.
4. If growth is justified, open a PR to raise the budget or update the baseline with a written rationale (see above).