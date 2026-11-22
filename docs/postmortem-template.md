# Incident Post-Mortem Template

Use this template for every production incident that causes user-visible impact or requires an emergency response. Complete all sections within 48 hours of resolution.

> **How to use:** copy this file to `docs/postmortems/YYYY-MM-DD-<slug>.md`, fill in each section, and open a PR for team review. Blameless culture — focus on systems and processes, not individuals.
>
> **Action items:** every action item in this postmortem must be filed as a GitHub issue labelled `postmortem` and linked in the Action Items table below. See the postmortem process in [docs/runbook.md](./runbook.md) for details.
>
> **Example:** See [docs/postmortems/2024-03-15-rpc-outage.md](./postmortems/2024-03-15-rpc-outage.md) for a fully completed hypothetical postmortem covering an RPC outage incident.

---

## Incident Summary

| Field | Value |
|-------|-------|
| **Incident ID** | INC-YYYY-NNN |
| **Date** | YYYY-MM-DD |
| **Severity** | P1 / P2 / P3 |
| **Status** | Resolved / Ongoing |
| **Duration** | X hours Y minutes |
| **Affected component(s)** | e.g. governance contract, token contract, frontend |
| **Incident commander** | @handle |
| **Authors** | @handle, @handle |

### Description

<!-- One paragraph: what happened, what users experienced, how it was detected. -->

---

## Timeline

All times in UTC — convert any local or log timestamps to UTC before filling in this table.

| Time (UTC) | Event |
|------------|-------|
| HH:MM | Incident begins / first alert fires |
| HH:MM | On-call engineer paged |
| HH:MM | Investigation started |
| HH:MM | Root cause identified |
| HH:MM | Mitigation applied (e.g. contract paused) |
| HH:MM | Service restored |
| HH:MM | Incident declared resolved |

---

## Root Cause Analysis

### What happened?

<!-- Technical explanation of the failure. Be specific: which function, which state transition, which edge case. -->

### Why did it happen?

<!-- Underlying cause — misconfiguration, logic bug, dependency failure, etc. Use the 5-Whys technique if helpful. -->

**5-Whys:**
1. Why? —
2. Why? —
3. Why? —
4. Why? —
5. Why? —

---

## Contributing Factors

List every condition that made the incident worse, harder to detect, or slower to resolve. Contributing factors are not necessarily root causes — they are the circumstances that shaped the impact and the response.

Consider:
- **Technical** — missing validation, brittle config, lack of circuit breaker, dependency behaviour
- **Detection** — gaps in alerting, noisy or missing dashboards, delayed event indexing
- **Process** — unclear runbook steps, missing escalation path, manual steps prone to error
- **Knowledge** — undocumented behaviour, single-owner components, assumptions that proved wrong

| # | Contributing factor | Category | Notes |
|---|---------------------|----------|-------|
| 1 | | Technical / Detection / Process / Knowledge | |
| 2 | | Technical / Detection / Process / Knowledge | |
| 3 | | Technical / Detection / Process / Knowledge | |

---

## Impact Assessment

| Dimension | Details |
|-----------|---------|
| **Users affected** | e.g. all voters during the window, proposers only |
| **Proposals affected** | IDs of any proposals whose state was incorrect or inaccessible |
| **Votes lost / corrupted** | number and addresses if known |
| **Financial impact** | e.g. none / estimated gas costs / token loss |
| **Data integrity** | on-chain state consistent / inconsistent — explain |
| **Regulatory / compliance** | any audit trail gaps |

---

## Action Items

Each item must have an owner and a due date, and must be tracked as a GitHub issue labelled `postmortem`. File the issue first, then link it in the table below so the postmortem and the tracking issue stay in sync.

| # | Action | Owner | Due date | GitHub issue | Status |
|---|--------|-------|----------|--------------|--------|
| 1 | | @handle | YYYY-MM-DD | #NNN | Open |
| 2 | | @handle | YYYY-MM-DD | #NNN | Open |
| 3 | | @handle | YYYY-MM-DD | #NNN | Open |

Action item categories to consider:
- **Prevent recurrence** — code fix, config change, additional validation
- **Improve detection** — new alert, monitoring dashboard, event index
- **Improve response** — runbook update, on-call training, escalation path
- **Reduce blast radius** — pause automation, circuit breaker, rate limiting

---

## Lessons Learned

### What went well?

<!-- Detection, communication, tooling, or response steps that worked effectively. -->

- 
- 

### What went poorly?

<!-- Gaps in tooling, process, knowledge, or communication that slowed the response. -->

- 
- 

### Where did we get lucky?

<!-- Near-misses or assumptions that happened to be correct this time. -->

- 
- 

---

## Appendix

### Relevant links

- Stellar Horizon event stream query: 
- Contract transaction hash(es): 
- GitHub issue / PR: 
- Monitoring dashboard snapshot: 

### Raw logs / evidence

```
# paste relevant log lines, contract error codes, or event payloads here
```
