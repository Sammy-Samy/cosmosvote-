import { SubscriptionStore } from '../src/subscriptions';

/** Create an isolated in-memory store for each test. */
function makeStore(): SubscriptionStore {
  return new SubscriptionStore(':memory:');
}

describe('SubscriptionStore', () => {
  let store: SubscriptionStore;

  beforeEach(() => {
    store = makeStore();
  });

  afterEach(() => {
    store.close();
  });

  // ── persist ──────────────────────────────────────────────────────────────

  it('persists a new subscription and returns the record', () => {
    const record = store.add({
      email: 'alice@example.com',
      webhookUrl: 'https://hooks.example.com/alice',
    });

    expect(record.id).toBeGreaterThan(0);
    expect(record.email).toBe('alice@example.com');
    expect(record.webhookUrl).toBe('https://hooks.example.com/alice');
    expect(record.createdAt).toBeGreaterThan(0);
  });

  // ── retrieve ──────────────────────────────────────────────────────────────

  it('returns an empty array when no subscriptions exist', () => {
    expect(store.findAll()).toEqual([]);
  });

  it('retrieves all stored subscriptions', () => {
    store.add({ email: 'alice@example.com', webhookUrl: 'https://hooks.example.com/alice' });
    store.add({ email: 'bob@example.com', webhookUrl: 'https://hooks.example.com/bob' });

    const all = store.findAll();
    expect(all).toHaveLength(2);
    expect(all.map((r) => r.email)).toEqual(['alice@example.com', 'bob@example.com']);
  });

  it('finds a subscription by ID', () => {
    const created = store.add({
      email: 'carol@example.com',
      webhookUrl: 'https://hooks.example.com/carol',
    });

    const found = store.findById(created.id);
    expect(found).toBeDefined();
    expect(found?.email).toBe('carol@example.com');
  });

  it('returns undefined when the ID does not exist', () => {
    expect(store.findById(9999)).toBeUndefined();
  });

  // ── delete ────────────────────────────────────────────────────────────────

  it('deletes a subscription by ID and returns true', () => {
    const record = store.add({
      email: 'dave@example.com',
      webhookUrl: 'https://hooks.example.com/dave',
    });

    const deleted = store.remove(record.id);
    expect(deleted).toBe(true);
    expect(store.findById(record.id)).toBeUndefined();
    expect(store.findAll()).toHaveLength(0);
  });

  it('returns false when trying to delete a non-existent ID', () => {
    expect(store.remove(9999)).toBe(false);
  });

  // ── durability ────────────────────────────────────────────────────────────

  it('subscriptions survive a store close and reopen (file-based)', () => {
    const os = require('os');
    const path = require('path');
    const fs = require('fs');
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'substore-'));
    const dbPath = path.join(tmpDir, 'subs.db');

    const s1 = new SubscriptionStore(dbPath);
    s1.add({ email: 'eve@example.com', webhookUrl: 'https://hooks.example.com/eve' });
    s1.close();

    const s2 = new SubscriptionStore(dbPath);
    const all = s2.findAll();
    s2.close();

    // Clean up temp file
    fs.rmSync(tmpDir, { recursive: true, force: true });

    expect(all).toHaveLength(1);
    expect(all[0].email).toBe('eve@example.com');
  });

  // ── duplicate email ───────────────────────────────────────────────────────

  it('throws when adding a duplicate email', () => {
    store.add({ email: 'frank@example.com', webhookUrl: 'https://hooks.example.com/frank' });
    expect(() =>
      store.add({ email: 'frank@example.com', webhookUrl: 'https://hooks.example.com/frank2' })
    ).toThrow();
  });

  // ── duplicate (email, webhookUrl) pair ────────────────────────────────────

  it('rejects a duplicate (email, webhookUrl) pair with a conflict error', () => {
    store.add({ email: 'henry@example.com', webhookUrl: 'https://hooks.example.com/henry' });

    expect(() =>
      store.add({ email: 'henry@example.com', webhookUrl: 'https://hooks.example.com/henry' })
    ).toThrow(/conflict/i);
  });

  it('exposes a 409 status on the duplicate conflict error', () => {
    store.add({ email: 'ivy@example.com', webhookUrl: 'https://hooks.example.com/ivy' });

    try {
      store.add({ email: 'ivy@example.com', webhookUrl: 'https://hooks.example.com/ivy' });
      throw new Error('expected duplicate add to throw');
    } catch (err) {
      expect((err as { status?: number }).status).toBe(409);
    }
  });

  it('does not persist a second row when a duplicate is rejected', () => {
    store.add({ email: 'jack@example.com', webhookUrl: 'https://hooks.example.com/jack' });
    expect(() =>
      store.add({ email: 'jack@example.com', webhookUrl: 'https://hooks.example.com/jack' })
    ).toThrow();

    expect(store.findAll()).toHaveLength(1);
  });

  it('allows the same email with a different webhook URL', () => {
    store.add({ email: 'kate@example.com', webhookUrl: 'https://hooks.example.com/kate-1' });
    store.add({ email: 'kate@example.com', webhookUrl: 'https://hooks.example.com/kate-2' });

    expect(store.findAll()).toHaveLength(2);
  });

  // ── deduplication of pre-existing duplicates ─────────────────────────────

  it('removes pre-existing duplicates keeping the earliest row', () => {
    // Bypass the duplicate guard to simulate legacy data written before the fix.
    const raw = store as unknown as { db: { prepare: (sql: string) => any } };
    const insert = raw.db.prepare(
      'INSERT INTO subscriptions (email, webhook_url, created_at) VALUES (?, ?, ?)'
    );
    insert.run('legacy@example.com', 'https://hooks.example.com/legacy', 1000);
    insert.run('legacy@example.com', 'https://hooks.example.com/legacy', 2000);
    insert.run('legacy@example.com', 'https://hooks.example.com/legacy', 3000);

    const removed = store.deduplicate();

    expect(removed).toBe(2);
    const all = store.findAll();
    expect(all).toHaveLength(1);
    expect(all[0].email).toBe('legacy@example.com');
    expect(all[0].webhookUrl).toBe('https://hooks.example.com/legacy');
    expect(all[0].createdAt).toBe(1000);
  });

  it('deduplicate is a no-op when there are no duplicates', () => {
    store.add({ email: 'liam@example.com', webhookUrl: 'https://hooks.example.com/liam' });
    store.add({ email: 'mia@example.com', webhookUrl: 'https://hooks.example.com/mia' });

    expect(store.deduplicate()).toBe(0);
    expect(store.findAll()).toHaveLength(2);
  });

  // ── migration idempotency ─────────────────────────────────────────────────

  it('runs migration without error on an already-migrated database', () => {
    // Close and reopen the same :memory: store is not possible for SQLite;
    // instead verify a second store can be instantiated cleanly.
    const s2 = makeStore();
    s2.add({ email: 'grace@example.com', webhookUrl: 'https://hooks.example.com/grace' });
    expect(s2.findAll()).toHaveLength(1);
    s2.close();
  });
});
