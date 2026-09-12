import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, anonContext, loginViaApi, CREATOR } from '../helpers/auth';
import { createPost, createSeries, createTier, uniqueSlug } from '../helpers/fixtures';
import { makeVerifiedUser, getFeedSeriesId, cleanupE2eRows, TestUser } from '../helpers/db';

let creatorCtx: APIRequestContext;
let fan: TestUser;
let gateTier: { id: string; name: string };
let uiSeries: { id: string; slug: string };

test.describe('creator dashboard', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    fan = await makeVerifiedUser('dash-fan');
    gateTier = await createTier(creatorCtx, 92, 500, 'dash-tier');
    uiSeries = await createSeries(creatorCtx, 'dash-series');
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    await creatorCtx.dispose();
  });

  test('create a tier via Settings -> Tiers UI and see it on /membership', async ({ page }) => {
    await loginViaApi(page, CREATOR.email, CREATOR.password);
    await page.goto('/settings');
    // Wait for the creator role to be resolved (creator sidebar appears)
    // before switching tabs, or a re-render can reset the active tab.
    await expect(page.getByRole('button', { name: 'Content' })).toBeVisible();
    await page.getByRole('tab', { name: 'Tiers' }).click();
    await page.getByRole('button', { name: 'Manage tiers' }).click();
    await page.getByRole('button', { name: 'Add tier' }).click();

    const name = uniqueSlug('ui-tier');
    await page.getByPlaceholder('Enter tier name').fill(name);
    await page.getByPlaceholder('Enter price').fill('11');
    await page.getByPlaceholder('Enter level').fill('93');
    const [createResp] = await Promise.all([
      page.waitForResponse(
        (r) => r.url().includes('/api/tiers') && r.request().method() === 'POST',
      ),
      page.getByRole('button', { name: 'Save' }).click(),
    ]);
    expect(createResp.status()).toBe(201);
    // The new tier shows up in the tiers list
    await expect(page.getByText(name).first()).toBeVisible();

    await page.goto('/membership');
    await expect(page.getByRole('heading', { name })).toBeVisible();
    await expect(page.getByText('$11.00')).toBeVisible();
  });

  test('create a gated post via the dashboard UI; locked for fan, unlocked for creator', async ({
    page,
    browser,
  }) => {
    await loginViaApi(page, CREATOR.email, CREATOR.password);
    await page.goto('/new-post');

    const title = uniqueSlug('ui-post');
    await page.getByPlaceholder('Enter post title...').fill(title);
    await page.getByLabel('Post Number').fill(String(100000 + Math.floor(Math.random() * 800000)));

    // Series select
    await page.getByRole('combobox').filter({ hasText: 'Select a series...' }).click();
    await page.getByRole('option', { name: uiSeries.slug }).click();

    // Minimum tier select
    await page.getByRole('combobox').filter({ hasText: 'Everyone' }).click();
    await page.getByRole('option', { name: gateTier.name }).click();

    // Article body (TinyMCE renders inside an iframe)
    const editorBody = page.frameLocator('.tox-edit-area iframe').locator('body');
    await editorBody.click();
    await editorBody.fill('Members-only UI post body');

    // Publish immediately
    await page.getByText('Publish this post immediately').click();
    await page.getByRole('button', { name: 'Create Post' }).click();
    await expect(page).not.toHaveURL(/\/new-post/);

    // Locked for a fan
    const fanContext = await browser.newContext();
    const fanPage = await fanContext.newPage();
    await loginViaApi(fanPage, fan.email, fan.password);
    await fanPage.goto(`/posts/${title}`);
    await expect(fanPage.getByText('This post is for members')).toBeVisible();
    await fanContext.close();

    // Unlocked for the creator
    const creatorView = await creatorCtx.get(`/proxy/api/public/posts/${title}`);
    expect(creatorView.ok()).toBeTruthy();
    const body = await creatorView.json();
    expect(body.access.granted).toBe(true);
    expect(body.access.reason).toBe('creator');
  });

  test('gated post via creator API is locked for a fan and unlocked for the creator', async ({
    page,
  }) => {
    const feedId = await getFeedSeriesId();
    const post = await createPost(creatorCtx, feedId, 'dash-gated', { minTierLevel: 2 });

    await loginViaApi(page, fan.email, fan.password);
    await page.goto(`/posts/${post.slug}`);
    await expect(page.getByText('This post is for members')).toBeVisible();

    const creatorView = await creatorCtx.get(`/proxy/api/public/posts/${post.slug}`);
    const body = await creatorView.json();
    expect(body.access.granted).toBe(true);
    expect(body.access.reason).toBe('creator');
  });

  test('creator can open /dashboard/content by direct URL', async ({ page }) => {
    await loginViaApi(page, CREATOR.email, CREATOR.password);
    await page.goto('/dashboard/content');
    await expect(page).toHaveURL(/\/dashboard\/content/);
    await expect(page.getByRole('tab', { name: 'Posts' })).toBeVisible();
  });

  test('deactivated tier disappears from /api/public/tiers', async () => {
    const tier = await createTier(creatorCtx, 94, 400, 'deact');
    const anon = await anonContext();

    const before = await anon.get('/proxy/api/public/tiers');
    expect((await before.json()).some((t: any) => t.id === tier.id)).toBe(true);

    const del = await creatorCtx.delete(`/proxy/api/tiers/${tier.id}`);
    expect(del.ok()).toBeTruthy();

    const after = await anon.get('/proxy/api/public/tiers');
    expect((await after.json()).some((t: any) => t.id === tier.id)).toBe(false);
    await anon.dispose();
  });
});
