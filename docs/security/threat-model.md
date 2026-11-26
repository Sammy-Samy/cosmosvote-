# Threat Model

## Threat Actors

### T1 — Malicious Voter
**Goal:** Cast more votes than balance entitles, or vote multiple times.  
**Mitigations:** `has_voted` guard, zero-balance check, vote weight captured immutably at vote time, flash-loan mitigation (loan repaid before tx ends → zero balance).  
**Residual Risk:** None

### T2 — Malicious Proposer
**Goal:** Create proposals that pass without genuine support, or spam the system with low-quality proposals.  
**Mitigations:** Quorum enforcement, `min_proposal_balance`, `proposal_cooldown`, admin cancel.  
**Production Recommendation:** Always set `min_proposal_balance` to a non-zero value in production (e.g., 1% of total supply) to prevent sybil spam.  
**Residual Risk:** Low

### T3 — Malicious Admin
**Goal:** Abuse privileged functions to manipulate outcomes.  
**Mitigations:** Admin can only cancel/execute — cannot alter votes or tallies. Admin vote restriction option. Events make all admin actions auditable.  
**Residual Risk:** Medium (by design — admin is a trusted role; use multisig in production)

### T3a — Malicious Treasury Signer / Governance Key Holder
**Goal:** Drain or redirect treasury funds by abusing the governance address that the treasury contract authorizes for disbursements.  
**Mitigations:** The treasury contract authorizes only the governance contract address, so every disbursement must flow through governance. In production the governance address is controlled by a 2-of-3 multisig (see `docs/admin-multisig-pattern.md`), which adds the following protections:
- **No single-key disbursement:** A single compromised or malicious signer cannot authorize a disbursement; at least two independent signers must approve.
- **Veto over disbursements:** Any one of the three signers can refuse to sign, blocking a disbursement that governance has approved. This gives each signer a unilateral veto on treasury outflows, so a governance proposal that passes on-chain still requires multisig co-signing before funds move.
- **Auditability:** Every multisig approval is an on-chain transaction, so disbursement approvals are traceable to individual signers.
**Residual Risk:** Medium. Two colluding signers (or two compromised signer keys) can approve any disbursement the governance contract authorizes, and a single compromised signer can block all disbursements (liveness veto). Mitigate by using independent key custody (separate hardware wallets, distinct operators), rotating signers, and monitoring for repeated vetoes or unusual approval patterns. A 3-of-5 or higher threshold reduces collusion risk at the cost of liveness.

### T4 — External Attacker (No Tokens)
**Goal:** Disrupt governance without holding tokens.  
**Mitigations:** `require_auth()` on all state-changing ops, initialization guard, state machine checks.  
**Residual Risk:** None

### T5 — Compromised Token Contract
**Goal:** Return inflated balances to favored voters.  
**Mitigations:** Admin must deploy a trustworthy token; token address is immutable after init.  
**Residual Risk:** High (external dependency — accepted)

### Reentrancy and Cross-Contract Calls
**Goal:** Prevent malicious token callbacks from manipulating governance tallies during vote casting or finalization.  
**Mitigations:** Soroban executes contract calls in a single deterministic transaction frame and enforces call stack isolation. Cross-contract callbacks during `cast_vote` are executed without allowing unsafe reentrant state mutations in the governance contract.  
**Residual Risk:** Low (verified by regression tests that deploy a malicious token contract and attempt to re-enter `cast_vote` through `balance_at`).

## RPC CORS Requirements
The frontend connects directly to a Soroban RPC endpoint. A browser-accessible RPC must expose a narrow CORS policy to avoid breaking the app while also preventing broad abuse.

Required headers for the RPC endpoint:
- `Access-Control-Allow-Origin: <frontend origin>` (do not use `*` for production)
- `Access-Control-Allow-Methods: POST, OPTIONS`
- `Access-Control-Allow-Headers: Content-Type`
- `Access-Control-Allow-Credentials: false`

For local Docker development, the repository uses a proxy service so the frontend can access the RPC at `http://localhost:8000` with explicit CORS headers while the underlying Soroban node remains isolated.

## Security Properties

| Property | Implementation |
|----------|---------------|
| Vote integrity | One vote per address; weight = live balance at vote time |
| Admin confinement | Admin cannot alter votes or tallies |
| Initialization safety | One-time init guard; admin/token immutable after |
| Arithmetic safety | `checked_add` on all vote accumulation |
| Finalization correctness | Pass condition evaluated atomically |
| Emergency response | Admin pause blocks all state-changing ops |
| Treasury disbursement control | Treasury authorizes only the governance address; production governance is a 2-of-3 multisig requiring two approvals per disbursement |
| Treasury veto | Any single multisig signer can withhold approval and block a disbursement (liveness veto) |
.
