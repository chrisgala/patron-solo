import { Pool } from 'pg';
import { randomUUID } from 'node:crypto';
import { BACKEND_URL } from '../playwright.config';

export const E2E_PREFIX = 'e2e-';

// One pool per worker process; all spec files share it, so it is never
// explicitly ended (the worker exiting closes the connections).
const pool = new Pool({
  host: 'localhost',
  port: 5432,
  user: 'postgres',
  password: 'password',
  database: 'postgres',
  max: 2,
  allowExitOnIdle: true,
});

export async function query<T = any>(sql: string, params: any[] = []): Promise<T[]> {
  const res = await pool.query(sql, params);
  return res.rows as T[];
}

export interface TestUser {
  id: string;
  email: string;
  password: string;
  displayName: string;
}

/** Unique e2e-prefixed email so fixtures never collide across runs. */
export function uniqueEmail(tag: string): string {
  return `${E2E_PREFIX}${tag}-${randomUUID().slice(0, 8)}@example.com`;
}

/**
 * Registers a user through the real API (argon2 hash, welcome email, etc.)
 * and then marks the email verified directly in Postgres.
 */
export async function makeVerifiedUser(
  tag: string,
  password = 'E2ePassw0rd!',
): Promise<TestUser> {
  const email = uniqueEmail(tag);
  const displayName = `${E2E_PREFIX}${tag}`;
  const res = await fetch(`${BACKEND_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password, displayName }),
  });
  if (!res.ok) {
    throw new Error(`register failed for ${email}: ${res.status} ${await res.text()}`);
  }
  const body = (await res.json()) as { user: { id: string } };
  await query('UPDATE users SET email_verified = true WHERE email = $1', [email]);
  return { id: body.user.id, email, password, displayName };
}

/**
 * Inserts a subscription row exactly as the Stripe webhook would.
 * tier_id is left null (allowed) so no tier row is required for the level.
 */
export async function giveSubscription(
  userId: string,
  tierLevel: number,
  status: 'active' | 'past_due' | 'canceled' = 'active',
  tierId: string | null = null,
): Promise<string> {
  const rows = await query<{ id: string }>(
    `INSERT INTO subscriptions (user_id, tier_id, tier_level, stripe_subscription_id, status, current_period_end)
     VALUES ($1, $2, $3, $4, $5, NOW() + interval '30 days')
     RETURNING id`,
    [userId, tierId, tierLevel, `${E2E_PREFIX}sub-${randomUUID()}`, status],
  );
  return rows[0].id;
}

/** Inserts a paid purchase row for a post XOR a series. */
export async function givePurchase(
  userId: string,
  target: { postId?: string; seriesId?: string },
  amountCents = 500,
): Promise<string> {
  const rows = await query<{ id: string }>(
    `INSERT INTO purchases (user_id, post_id, series_id, stripe_payment_intent_id, amount_cents, status)
     VALUES ($1, $2, $3, $4, $5, 'paid')
     RETURNING id`,
    [userId, target.postId ?? null, target.seriesId ?? null, `${E2E_PREFIX}pi-${randomUUID()}`, amountCents],
  );
  return rows[0].id;
}

/** Id of the hidden feed series every stack has. */
export async function getFeedSeriesId(): Promise<string> {
  const rows = await query<{ id: string }>(
    'SELECT id FROM series WHERE is_feed = true AND deleted_at IS NULL LIMIT 1',
  );
  if (!rows.length) throw new Error('No feed series found');
  return rows[0].id;
}

export async function setFreeAt(postId: string, freeAt: Date | null): Promise<void> {
  // posts.free_at is `timestamp without time zone` holding UTC; pass an ISO
  // string and strip the offset explicitly so the client TZ can't skew it.
  await query(
    "UPDATE posts SET free_at = ($2::timestamptz AT TIME ZONE 'utc') WHERE id = $1",
    [postId, freeAt ? freeAt.toISOString() : null],
  );
}

/**
 * Deletes only rows this suite created (e2e- prefixed emails/slugs/names and
 * stripe ids). Never truncates shared tables.
 */
export async function cleanupE2eRows(): Promise<void> {
  await query(`DELETE FROM purchases WHERE stripe_payment_intent_id LIKE $1`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM subscriptions WHERE stripe_subscription_id LIKE $1`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM push_subscriptions WHERE endpoint LIKE $1`, [`%${E2E_PREFIX}%`]);
  await query(`DELETE FROM posts WHERE slug LIKE $1`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM user_files WHERE original_filename LIKE $1`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM series WHERE slug LIKE $1 AND is_feed = false`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM tiers WHERE name LIKE $1`, [`${E2E_PREFIX}%`]);
  await query(`DELETE FROM users WHERE email LIKE $1`, [`${E2E_PREFIX}%`]);
}
