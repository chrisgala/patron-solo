import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, anonContext, CREATOR } from '../helpers/auth';
import { createPost } from '../helpers/fixtures';
import { getFeedSeriesId, setFreeAt, cleanupE2eRows } from '../helpers/db';

let creatorCtx: APIRequestContext;
let anon: APIRequestContext;
let post: { id: string; slug: string };

test.describe('rolling paywall', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    anon = await anonContext();
    const feedId = await getFeedSeriesId();
    post = await createPost(creatorCtx, feedId, 'roll', {
      minTierLevel: 2,
      freeAt: new Date(Date.now() + 7 * 24 * 3600 * 1000).toISOString(),
      content: '<p>rolling paywall content</p>',
    });
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    await creatorCtx.dispose();
    await anon.dispose();
  });

  test('post with future freeAt is locked, then unlocks when freeAt passes', async ({ page }) => {
    // Locked while freeAt is in the future
    const before = await anon.get(`/proxy/api/public/posts/${post.slug}`);
    const beforeBody = await before.json();
    expect(beforeBody.access.granted).toBe(false);
    expect(beforeBody.content).toBeNull();
    expect(beforeBody.access.freeAt).toBeTruthy();

    // The UI shows the paywall with the future free date
    await page.goto(`/posts/${post.slug}`);
    await expect(page.getByText('This post is for members')).toBeVisible();
    await expect(page.getByText(/Free on /)).toBeVisible();

    // Roll the paywall: only free_at changes, via SQL as time passing would
    await setFreeAt(post.id, new Date(Date.now() - 60 * 1000));

    const after = await anon.get(`/proxy/api/public/posts/${post.slug}`);
    const afterBody = await after.json();
    expect(afterBody.access.granted).toBe(true);
    expect(afterBody.access.reason).toBe('free');
    expect(afterBody.content).toContain('rolling paywall content');

    await page.goto(`/posts/${post.slug}`);
    await expect(page.getByText('rolling paywall content')).toBeVisible();
  });
});
