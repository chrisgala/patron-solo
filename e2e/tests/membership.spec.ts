import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, CREATOR, loginViaApi } from '../helpers/auth';
import { createTier } from '../helpers/fixtures';
import { makeVerifiedUser, giveSubscription, cleanupE2eRows, TestUser } from '../helpers/db';

let creatorCtx: APIRequestContext;
let lowTier: { id: string; name: string };
let highTier: { id: string; name: string };
let fan: TestUser;
let subscribedFan: TestUser;

test.describe('membership page', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    lowTier = await createTier(creatorCtx, 90, 900, 'member-low');
    highTier = await createTier(creatorCtx, 91, 1900, 'member-high');
    fan = await makeVerifiedUser('member-fan');
    subscribedFan = await makeVerifiedUser('member-sub');
    await giveSubscription(subscribedFan.id, 90, 'active', lowTier.id);
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    await creatorCtx.dispose();
  });

  test('tiers are listed ordered by level with their prices', async ({ page }) => {
    await page.goto('/membership');
    await expect(page.getByRole('heading', { name: 'Membership' })).toBeVisible();

    const lowCard = page.locator('div.relative', { has: page.getByRole('heading', { name: lowTier.name }) });
    await expect(page.getByRole('heading', { name: lowTier.name })).toBeVisible();
    await expect(page.getByRole('heading', { name: highTier.name })).toBeVisible();
    await expect(lowCard.getByText('$9.00').first()).toBeVisible();

    // DOM order must follow tier level
    const names = await page.locator('h3').allTextContents();
    expect(names.indexOf(lowTier.name)).toBeGreaterThanOrEqual(0);
    expect(names.indexOf(lowTier.name)).toBeLessThan(names.indexOf(highTier.name));
  });

  test('anonymous Join redirects to /register', async ({ page }) => {
    await page.goto('/membership');
    const card = page.locator('div', { has: page.getByRole('heading', { name: lowTier.name }) });
    await card.getByRole('button', { name: 'Join' }).first().click();
    await expect(page).toHaveURL(/\/register$/);
  });

  test('logged-in Join surfaces the Stripe-not-configured error gracefully', async ({ page }) => {
    await loginViaApi(page, fan.email, fan.password);
    await page.goto('/membership');
    const joinButton = page
      .locator('div', { has: page.getByRole('heading', { name: lowTier.name }) })
      .getByRole('button', { name: 'Join' })
      .first();
    await joinButton.click();
    // No crash, no navigation to Stripe; the page stays functional.
    await expect(page).toHaveURL(/\/membership$/);
    await expect(page.getByRole('heading', { name: 'Membership' })).toBeVisible();
    // The backend rejects with a Stripe-not-configured error.
    const res = await page.request.post('/proxy/api/billing/subscribe', {
      data: { tierId: lowTier.id },
    });
    expect(res.status()).toBe(400);
    expect((await res.text())).toContain('Stripe is not configured');
  });

  test('fan with an active subscription sees the current-plan state', async ({ page }) => {
    await loginViaApi(page, subscribedFan.email, subscribedFan.password);
    await page.goto('/membership');
    await expect(page.getByText('You are subscribed at tier level 90')).toBeVisible();
    const card = page.locator('div', { has: page.getByRole('heading', { name: lowTier.name }) });
    await expect(card.getByText('Current plan').first()).toBeVisible();
  });
});
