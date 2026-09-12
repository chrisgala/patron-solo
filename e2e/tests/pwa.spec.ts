import { test, expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { query } from '../helpers/db';

test.describe('PWA and web push plumbing', () => {
  test('manifest.webmanifest is valid JSON with icons', async ({ request }) => {
    const res = await request.get('/manifest.webmanifest');
    expect(res.status()).toBe(200);
    const manifest = JSON.parse(await res.text());
    expect(manifest.name || manifest.short_name).toBeTruthy();
    expect(Array.isArray(manifest.icons)).toBe(true);
    expect(manifest.icons.length).toBeGreaterThan(0);
    expect(manifest.icons[0].src).toBeTruthy();
  });

  test('service worker script is served as JavaScript', async ({ request }) => {
    const res = await request.get('/sw.js');
    expect(res.status()).toBe(200);
    expect(res.headers()['content-type']).toContain('javascript');
    expect((await res.text()).length).toBeGreaterThan(0);
  });

  test('GET /api/public/push/key returns the VAPID public key', async ({ request }) => {
    const res = await request.get('/proxy/api/public/push/key');
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(typeof body.publicKey).toBe('string');
    expect(body.publicKey.length).toBeGreaterThan(20);
  });

  test('push subscribe then unsubscribe round-trips with 204', async ({ request }) => {
    const endpoint = `https://example.com/fake-e2e-${randomUUID()}`;
    const sub = await request.post('/proxy/api/push/subscribe', {
      data: { endpoint, p256dh: 'x', auth: 'y' },
    });
    expect(sub.status()).toBe(204);

    const rows = await query('SELECT id FROM push_subscriptions WHERE endpoint = $1', [endpoint]);
    expect(rows.length).toBe(1);

    const unsub = await request.delete('/proxy/api/push/subscribe', {
      data: { endpoint },
    });
    expect(unsub.status()).toBe(204);

    const after = await query('SELECT id FROM push_subscriptions WHERE endpoint = $1', [endpoint]);
    expect(after.length).toBe(0);
  });
});
