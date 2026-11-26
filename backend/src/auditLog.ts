import * as fs from 'fs';
import * as path from 'path';
import * as zlib from 'zlib';

/**
 * Structured audit log entry for admin-level backend operations.
 *
 * All fields are required so that every admin operation produces a
 * complete, tamper-evident record.
 */
export interface AuditLogEntry {
  /** ISO-8601 timestamp of when the operation was recorded. */
  timestamp: string;
  /** Address of the admin who performed the operation. */
  adminAddress: string;
  /** The admin action that was performed (e.g. 'cancel', 'execute', 'pause', 'transfer_admin'). */
  action: string;
  /** Identifier of the proposal the action relates to, if applicable. */
  proposalId: string;
  /** Outcome of the operation (e.g. 'success', 'failure', 'rejected'). */
  outcome: string;
}

/**
 * Options for configuring the audit log module.
 */
export interface AuditLogOptions {
  /** Absolute or relative path to the append-only audit log file. */
  logFilePath?: string;
  /** Maximum size in bytes of the active log file before rotation. */
  maxBytes?: number;
  /** Number of days rotated (gzipped) log files are retained. */
  retentionDays?: number;
}

const DEFAULT_LOG_FILE = path.resolve(
  process.env.AUDIT_LOG_PATH || path.join(process.cwd(), 'logs', 'admin-audit.log'),
);

/** Default maximum size of the active log file before rotation (100 MB). */
const DEFAULT_MAX_BYTES = 100 * 1024 * 1024;

/** Default number of days rotated (gzipped) log files are retained. */
const DEFAULT_RETENTION_DAYS = 30;

/**
 * Parse a positive integer from an environment variable, falling back to the
 * provided default when the value is missing or invalid.
 */
function parsePositiveInt(value: string | undefined, fallback: number): number {
  if (value === undefined || value === '') {
    return fallback;
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return fallback;
  }
  return Math.floor(parsed);
}

/** Maximum size in bytes of the active log file before it is rotated. */
let maxBytes: number = parsePositiveInt(process.env.AUDIT_LOG_MAX_BYTES, DEFAULT_MAX_BYTES);

/** Number of days rotated log files are retained before being pruned. */
let retentionDays: number = parsePositiveInt(
  process.env.AUDIT_LOG_RETENTION_DAYS,
  DEFAULT_RETENTION_DAYS,
);

let logFilePath: string = DEFAULT_LOG_FILE;
let initialized = false;

/** Calendar date (YYYY-MM-DD) on which the active log file was last written. */
let activeLogDate: string | null = null;

/**
 * Ensure the directory for the audit log file exists and that the file is
 * opened in append mode. The file is never truncated or rewritten.
 */
function ensureInitialized(): void {
  if (initialized) {
    return;
  }

  const dir = path.dirname(logFilePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  // Open with O_APPEND to guarantee that every write is appended atomically
  // to the end of the file. The file is created if it does not exist.
  const fd = fs.openSync(logFilePath, 'a');
  fs.closeSync(fd);

  activeLogDate = currentDateString();

  initialized = true;
}

/** Returns the current local calendar date as a YYYY-MM-DD string. */
function currentDateString(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/** Returns the size of the active log file in bytes, or 0 if it is missing. */
function getActiveLogSize(): number {
  try {
    return fs.statSync(logFilePath).size;
  } catch {
    return 0;
  }
}

/**
 * Determine whether the active log file should be rotated, either because it
 * has exceeded the configured size threshold or because the calendar date has
 * changed since the last write.
 */
function shouldRotate(): boolean {
  if (getActiveLogSize() >= maxBytes) {
    return true;
  }
  if (activeLogDate !== null && activeLogDate !== currentDateString()) {
    return true;
  }
  return false;
}

/**
 * Rotate the active log file: rename it with a timestamp suffix, gzip it
 * asynchronously, and prune rotated files older than the retention period.
 */
function rotateLog(): void {
  if (!fs.existsSync(logFilePath)) {
    activeLogDate = currentDateString();
    return;
  }

  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const rotatedPath = `${logFilePath}.${timestamp}`;

  try {
    fs.renameSync(logFilePath, rotatedPath);
  } catch {
    // If the rename fails (e.g. a concurrent rotation), keep appending to the
    // existing file rather than losing entries.
    return;
  }

  // Recreate the active log file so subsequent appends have a target.
  const fd = fs.openSync(logFilePath, 'a');
  fs.closeSync(fd);

  activeLogDate = currentDateString();

  // Compress the rotated file asynchronously so the write path is not blocked.
  const gzipPath = `${rotatedPath}.gz`;
  const readStream = fs.createReadStream(rotatedPath);
  const writeStream = fs.createWriteStream(gzipPath);
  const gzip = zlib.createGzip();

  readStream
    .pipe(gzip)
    .pipe(writeStream)
    .on('finish', () => {
      try {
        fs.unlinkSync(rotatedPath);
      } catch {
        // Ignore failures removing the uncompressed rotated file.
      }
      pruneOldLogs();
    })
    .on('error', () => {
      // Leave the uncompressed rotated file in place if compression fails.
    });
}

/**
 * Remove gzipped rotated log files that are older than the retention period.
 */
function pruneOldLogs(): void {
  const dir = path.dirname(logFilePath);
  const base = path.basename(logFilePath);
  const cutoff = Date.now() - retentionDays * 24 * 60 * 60 * 1000;

  let files: string[];
  try {
    files = fs.readdirSync(dir);
  } catch {
    return;
  }

  for (const file of files) {
    if (!file.startsWith(`${base}.`) || !file.endsWith('.gz')) {
      continue;
    }
    const fullPath = path.join(dir, file);
    try {
      const stat = fs.statSync(fullPath);
      if (stat.mtimeMs < cutoff) {
        fs.unlinkSync(fullPath);
      }
    } catch {
      // Ignore files that disappear or cannot be inspected.
    }
  }
}

/**
 * Configure the audit log module. Intended to be called once at startup.
 * Changing the log file path after initialization is not supported to keep
 * the log append-only and tamper-evident.
 */
export function configureAuditLog(options: AuditLogOptions): void {
  if (initialized) {
    throw new Error('Audit log has already been initialized and cannot be reconfigured');
  }
  if (options.logFilePath) {
    logFilePath = path.resolve(options.logFilePath);
  }
  if (options.maxBytes !== undefined) {
    maxBytes = options.maxBytes;
  }
  if (options.retentionDays !== undefined) {
    retentionDays = options.retentionDays;
  }
}

/**
 * Record an admin action in the append-only audit log.
 *
 * This is the only write API exposed by this module. There are intentionally
 * no update or delete APIs: entries can only ever be appended.
 *
 * @param adminAddress Address of the admin performing the action.
 * @param action The admin action being performed.
 * @param proposalId Identifier of the related proposal (use an empty string
 *   or a sentinel such as 'n/a' when the action is not proposal-scoped).
 * @param outcome The outcome of the operation.
 * @returns The recorded audit log entry.
 */
export function recordAdminAction(
  adminAddress: string,
  action: string,
  proposalId: string,
  outcome: string,
): AuditLogEntry {
  if (!adminAddress) {
    throw new Error('Audit log entry requires an adminAddress');
  }
  if (!action) {
    throw new Error('Audit log entry requires an action');
  }
  if (!outcome) {
    throw new Error('Audit log entry requires an outcome');
  }

  ensureInitialized();

  if (shouldRotate()) {
    rotateLog();
  }

  const entry: AuditLogEntry = {
    timestamp: new Date().toISOString(),
    adminAddress,
    action,
    proposalId: proposalId ?? '',
    outcome,
  };

  const line = JSON.stringify(entry) + '\n';

  // O_APPEND guarantees atomic append semantics; each entry is written as a
  // single newline-delimited JSON record.
  const fd = fs.openSync(logFilePath, 'a');
  try {
    fs.writeSync(fd, line, null, 'utf8');
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }

  activeLogDate = currentDateString();

  return entry;
}

/**
 * Read all audit log entries. Provided for verification/inspection only;
 * there is no API to modify or remove entries.
 */
export function readAuditLog(): AuditLogEntry[] {
  ensureInitialized();

  if (!fs.existsSync(logFilePath)) {
    return [];
  }

  const content = fs.readFileSync(logFilePath, 'utf8');
  const entries: AuditLogEntry[] = [];

  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    try {
      entries.push(JSON.parse(trimmed) as AuditLogEntry);
    } catch {
      // Skip malformed lines rather than failing the whole read.
    }
  }

  return entries;
}

/**
 * Returns the path of the audit log file currently in use.
 */
export function getAuditLogPath(): string {
  return logFilePath;
}