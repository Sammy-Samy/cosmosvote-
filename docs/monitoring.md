# Runtime Monitoring for Anomalous Contract Events

Off-chain monitoring script that polls Soroban contract events and emits structured alerts when suspicious governance or token activity is detected.

## Quick Start

```bash
export STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
export GOVERNANCE_CONTRACT_ID="C..."
export TOKEN_CONTRACT_ID="C..."

node scripts/monitor.js
```

Output is newline-delimited JSON streamed to stdout, suitable for log aggregators (Datadog, CloudWatch, Loki, etc.).

## Anomalies Detected

| Anomaly | Description | Default Threshold |
|---------|-------------|-------------------|
| `SUSPICIOUS_ADMIN_CHANGES` | Multiple `set_admin` or `update_config` events in one poll window | ≥ 2 events |
| `LARGE_TOKEN_TRANSFER` | Single transfer above threshold | ≥ 1,000,000,000,000 base units |
| `RAPID_VOTE_BURST` | Unusually high vote volume in one poll window | ≥ 50 votes |
| `PROPOSAL_CANCELLED` | Any admin cancellation of a governance proposal | Every occurrence |
| `CONTRACT_PAUSED` | Governance contract pause event | Every occurrence |
| `TOKEN_MINTED` | Any mint event (unexpected supply increase) | Every occurrence |

## Configuration

All settings are controlled via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `STELLAR_RPC_URL` | `https://soroban-testnet.stellar.org` | Soroban JSON-RPC endpoint |
| `GOVERNANCE_CONTRACT_ID` | _(empty)_ | Governance contract address |
| `TOKEN_CONTRACT_ID` | _(empty)_ | Token contract address |
| `POLL_INTERVAL_MS` | `30000` | Milliseconds between polls |
| `LARGE_TRANSFER_THRESHOLD` | `1000000000000` | Base-unit threshold for large-transfer alerts |
| `RAPID_VOTE_BURST` | `50` | Vote-count threshold per poll window |
| `ADMIN_CHANGE_COUNT` | `2` | Admin-change count before alert |

## Alert Output Format

Every log line is a JSON object:

```json
{
  "timestamp": "2026-01-15T10:00:00.000Z",
  "level": "ALERT",
  "event": "LARGE_TOKEN_TRANSFER",
  "contractId": "C...",
  "from": "G...",
  "to": "G...",
  "amount": "5000000000000",
  "threshold": "1000000000000"
}
```

`level` is one of `INFO`, `ALERT`, or `ERROR`.

## Adding Alerting Hooks

The `alert()` function in `scripts/monitor.js` is the single extension point. Add integrations there:

```js
// Slack
fetch(process.env.SLACK_WEBHOOK_URL, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ text: `[ALERT] ${event}: ${JSON.stringify(details)}` }),
});

// PagerDuty, email, SNS, etc.
```

## Log Aggregation

Backend containers write newline-delimited JSON to stdout. Docker Compose ships with a `logging` driver configured for every backend service so logs survive container restarts and are queryable centrally.

### Logging Driver

Two drivers are supported; pick one via `LOG_DRIVER` in your `.env`:

| `LOG_DRIVER` | Driver | Notes |
|--------------|--------|-------|
| `awslogs` (default) | Amazon CloudWatch Logs | Requires `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (or an instance role). Log group: `${LOG_GROUP_PREFIX:-cosmosvote}/backend`. |
| `loki` | Grafana Loki | Requires the `loki` Docker logging plugin (`docker plugin install grafana/loki-docker-driver:latest --alias loki --grant-all-permissions`) and `LOKI_URL`. |

Example Compose snippet (already applied to backend services):

```yaml
logging:
  driver: ${LOG_DRIVER:-awslogs}
  options:
    awslogs-region: ${AWS_REGION:-us-east-1}
    awslogs-group: ${LOG_GROUP_PREFIX:-cosmosvote}/backend
    awslogs-stream: ${SERVICE_NAME:-backend}-{{.Name}}
    awslogs-create-group: "true"
    # Loki alternative:
    # loki-url: ${LOKI_URL}
    # loki-retries: "3"
    # loki-batch-wait: "1s"
```

### Retention Policy

- **Minimum retention: 30 days** for all backend log streams.
- CloudWatch: set the log group retention to `30` (or longer) days:
  ```bash
  aws logs put-retention-policy \
    --log-group-name "${LOG_GROUP_PREFIX:-cosmosvote}/backend" \
    --retention-in-days 30
  ```
- Loki: configure the `limits_config.retention_period: 720h` (30 days) in the Loki config, or a longer period for compliance needs.
- Retention is enforced by the aggregation backend, not by Docker; local `json-file` rotation is disabled for aggregated services.

### Querying Logs

CloudWatch Logs Insights:

```
fields @timestamp, level, service, event, traceId, err
| filter level = "error"
| sort @timestamp desc
| limit 100
```

Loki (LogQL):

```logql
{service="backend"} | json | level="error"
{service="backend"} | json | event="LARGE_TOKEN_TRANSFER"
{service="backend"} | json | traceId="<trace-id>"
```

### Structured Log Fields

Emitted by `backend/src/logger.ts`. All lines are JSON objects with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | string (ISO 8601) | Time the record was emitted |
| `level` | string | `debug`, `info`, `warn`, or `error` |
| `service` | string | Service name (e.g. `backend`) |
| `pid` | number | Process ID of the emitting worker |
| `traceId` | string | Request/correlation ID for tracing across services |
| `event` | string | Logical event name (e.g. `LARGE_TOKEN_TRANSFER`) |
| `err` | object | Serialized error (`message`, `stack`, `name`) when `level=error` |
| `webhookUrl` | string | **Redacted** — replaced with `[REDACTED]` |
| `email` | string | **Redacted** |
| `secretKey` | string | **Redacted** |
| `password` | string | **Redacted** |
| `token` | string | **Redacted** |
| `apiKey` | string | **Redacted** |
| `authorization` | string | **Redacted** |
| `dsn` | string | **Redacted** |

Sensitive fields are scrubbed by the logger before serialization; never log raw credentials. Use `traceId` to correlate a request across services and `event` for alert-oriented queries.

## Running in Production

For continuous operation, run the script under a process manager:

```bash
# PM2
pm2 start scripts/monitor.js --name cosmosvote-monitor

# systemd — see your distro documentation
# Docker
docker run -e STELLAR_RPC_URL=... -e GOVERNANCE_CONTRACT_ID=... node:20-alpine node scripts/monitor.js
```

## Tuning Thresholds

Adjust thresholds based on observed on-chain activity:

1. Run the monitor in `INFO`-only mode for a week to collect baseline metrics.
2. Set `LARGE_TRANSFER_THRESHOLD` to ~10× the typical single-transaction volume.
3. Set `RAPID_VOTE_BURST` to ~3× the typical votes-per-30s rate.
4. Lower `ADMIN_CHANGE_COUNT` to `1` on mainnet for stricter alerting.
