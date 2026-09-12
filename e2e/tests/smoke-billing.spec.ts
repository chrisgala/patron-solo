import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, CREATOR, loginViaApi } from '../helpers/auth';
import { createPost } from '../helpers/fixtures';
import {
  makeVerifiedUser,
  giveSubscription,
  givePurchase,
  getFeedSeriesId,
  cleanupE2eRows,
  TestUser,
} from '../helpers/db';

let creatorCtx: APIRequestContext;
let fan: TestUser;
let pricedPost: { id: string; slug: string };

test.describe('billing smoke', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    const feedId = await getFeedSeriesId();
    pricedPost = await createPost(creatorCtx, feedId, 'billing-priced', { priceCents: 800 });
    fan = await makeVerifiedUser('billing-fan');
    await giveSubscription(fan.id, 2, 'active');
    await givePurchase(fan.id, { postId: pricedPost.id }, 800);
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    await creatorCtx.dispose();
  });

  test('/billing/success page renders', async ({ page }) => {
    await loginViaApi(page, fan.email, fan.password);
    await page.goto('/billing/success');
    // The fan has a subscription + purchase, so polling confirms immediately.
    await expect(page.getByRole('heading', { name: 'Payment confirmed' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Back to the feed' })).toBeVisible();
  });

  test('/billing/cancel page renders', async ({ page }) => {
    await page.goto('/billing/cancel');
    await expect(page.getByRole('heading', { name: 'Checkout cancelled' })).toBeVisible();
    await expect(page.getByText('No payment was made.')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Back to the feed' })).toBeVisible();
  });

  test('GET /api/billing/me reflects the seeded subscription and purchase', async ({ page }) => {
    await loginViaApi(page, fan.email, fan.password);
    const res = await page.request.get('/proxy/api/billing/me');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.subscription).toBeTruthy();
    expect(body.subscription.tierLevel).toBe(2);
    expect(body.subscription.status).toBe('active');
    const purchase = body.purchases.find((p: any) => p.postId === pricedPost.id);
    expect(purchase).toBeTruthy();
    expect(purchase.amountCents).toBe(800);
  });
});
