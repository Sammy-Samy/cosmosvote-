import { loadEnv } from './env';
import { initErrorTracking, flushErrorTracking } from './errorTracking';
import { createHealthServer } from './health';
import { NotificationService } from './notificationService';
import { logger } from './logger';
import helmet from 'helmet';
import { readFileSync } from 'fs';
import { join } from 'path';

// Resolve the running version. Prefer an explicitly injected version (set by
// the release workflow at build time), falling back to package.json.
function resolveVersion(): string {
  if (process.env.COSMOSVOTE_VERSION) {
    return process.env.COSMOSVOTE_VERSION;
  }
  try {
    const pkgPath = join(__dirname, '..', 'package.json');
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8')) as { version?: string };
    return pkg.version ?? '0.0.0';
  } catch {
    return '0.0.0';
  }
}

const APP_VERSION = resolveVersion();

// Validate env vars before anything else — fails fast with a clear message.
const env = loadEnv();

// Must be called before any async work so unhandled rejections are captured.
initErrorTracking();

// Initialize the append-only audit log for admin operations.
auditLogger.init({ logPath: env.AUDIT_LOG_PATH });

const service = new NotificationService({
  rpcUrl: env.STELLAR_RPC_URL,
  contractId: env.GOVERNANCE_CONTRACT_ID,
  webhookUrl: env.WEBHOOK_URL,
  pollIntervalMs: env.POLL_INTERVAL_MS,
});

// Security headers middleware. Applied to every HTTP response.
// HTTPS-only enforcement: HSTS (max-age=31536000) instructs browsers to
// only use HTTPS for this host for one year. The API must be served over
// TLS (terminated at the load balancer / reverse proxy) for HSTS to take
// effect; plain-HTTP deployments should redirect to HTTPS upstream.
const securityHeaders = helmet({
  hsts: { maxAge: 31536000, includeSubDomains: true },
  frameguard: { action: 'deny' },
  noSniff: true,
  referrerPolicy: { policy: 'strict-origin' },
});

const healthServer = createHealthServer(() => service.checkReadiness(), securityHeaders);

// Expose the running version so deployments can verify what is live.
healthServer.on('request', (req, res) => {
  if (req.method === 'GET' && req.url === '/version') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ version: APP_VERSION }));
  }
});

const healthPort = env.HEALTH_PORT;
healthServer.listen(healthPort, '0.0.0.0', () => {
  console.info(
    `[cosmosvote-backend] health endpoints listening on port ${healthPort} (version ${APP_VERSION})`,
  );
});

service.start();
logger.info({ event: 'startup' }, 'notification service started');

async function shutdown() {
  logger.info({ event: 'shutdown' }, 'notification service stopping');
  service.stop();
  healthServer.close();
  await auditLogger.flush();
  await flushErrorTracking();
  process.exit(0);
}

process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);