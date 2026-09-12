import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, CREATOR } from '../helpers/auth';
import { createPost } from '../helpers/fixtures';
import { getFeedSeriesId, cleanupE2eRows } from '../helpers/db';

let creatorCtx: APIRequestContext;
let freePost: { id: string; slug: string };
let gatedPost: { id: string; slug: string };
let pricedPost: { id: string; slug: string };
let rolledFreePost: { id: string; slug: string };

test.describe('public home page (anonymous)', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    const feedId = await getFeedSeriesId();
    freePost = await createPost(creatorCtx, feedId, 'home-free', {
      content: '<p>This free article body should appear as an excerpt on the card.</p>',
    });
    gatedPost = await createPost(creatorCtx, feedId, 'home-gated', { minTierLevel: 2 });
    pricedPost = await createPost(creatorCtx, feedId, 'home-priced', { priceCents: 700 });
    rolledFreePost = await createPost(creatorCtx, feedId, 'home-rolled', {
      minTierLevel: 1,
      freeAt: new Date(Date.now() - 24 * 3600 * 1000).toISOString(),
      content: '<p>Once gated, now free rolled content.</p>',
    });
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    await creatorCtx.dispose();
  });

  test('renders the creator header banner', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Log in' })).toBeVisible();
  });

  test('free post card shows excerpt and Read more', async ({ page }) => {
    await page.goto('/');
    const card = page.locator('article', { hasText: freePost.slug });
    await expect(card).toBeVisible();
    await expect(card.getByText('This free article body should appear')).toBeVisible();
    await expect(card.getByRole('link', { name: 'Read more' })).toBeVisible();
  });

  test('tier-gated card shows lock and Join tier CTA', async ({ page }) => {
    await page.goto('/');
    const card = page.locator('article', { hasText: gatedPost.slug });
    await expect(card).toBeVisible();
    await expect(card.getByText('This post is for members')).toBeVisible();
    await expect(card.getByRole('button', { name: 'Join tier 2' })).toBeVisible();
    // No content link for a locked card
    await expect(card.getByRole('link', { name: 'Read more' })).toHaveCount(0);
  });

  test('priced card shows Buy for $X CTA', async ({ page }) => {
    await page.goto('/');
    const card = page.locator('article', { hasText: pricedPost.slug });
    await expect(card).toBeVisible();
    await expect(card.getByRole('button', { name: 'Buy for $7.00' })).toBeVisible();
  });

  test('rolled-free post is unlocked and shows its content', async ({ page }) => {
    await page.goto('/');
    const card = page.locator('article', { hasText: rolledFreePost.slug });
    await expect(card).toBeVisible();
    await expect(card.getByText('Once gated, now free rolled content.')).toBeVisible();
    await expect(card.getByRole('link', { name: 'Read more' })).toBeVisible();

    await page.goto(`/posts/${rolledFreePost.slug}`);
    await expect(page.getByText('Once gated, now free rolled content.')).toBeVisible();
  });

  test('navigating to a locked post URL shows the paywall panel', async ({ page }) => {
    await page.goto(`/posts/${gatedPost.slug}`);
    await expect(page.getByText('This post is for members')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Join tier 2' })).toBeVisible();
    // Anonymous visitor gets a login hint
    await expect(page.getByText('Already a member?')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Log in' }).last()).toBeVisible();
  });
});
