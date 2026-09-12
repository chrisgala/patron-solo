import { test, expect } from '@playwright/test';
import { makeVerifiedUser, cleanupE2eRows, TestUser } from '../helpers/db';
import { loginViaApi, CREATOR } from '../helpers/auth';

let fan: TestUser;

test.describe('fan vs creator navigation', () => {
  test.beforeAll(async () => {
    fan = await makeVerifiedUser('nav-fan');
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
  });

  test('fan visiting /dashboard/content is bounced to /', async ({ page }) => {
    await loginViaApi(page, fan.email, fan.password);
    await page.goto('/dashboard/content');
    await expect(page).toHaveURL('http://localhost:5173/');
  });

  test('fan nav lacks Dashboard; sidebar shows fan items', async ({ page }) => {
    await loginViaApi(page, fan.email, fan.password);
    await page.goto('/');
    await expect(page.getByRole('link', { name: 'Home' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Settings' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Dashboard' })).toHaveCount(0);

    // Sidebar on the settings page shows fan items only
    await page.goto('/settings');
    await expect(page.getByRole('button', { name: 'Membership' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Dashboard' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Content' })).toHaveCount(0);
  });

  test('creator nav has Dashboard and can open it', async ({ page }) => {
    await loginViaApi(page, CREATOR.email, CREATOR.password);
    await page.goto('/');
    const dashboardLink = page.getByRole('link', { name: 'Dashboard' });
    await expect(dashboardLink).toBeVisible();
    await dashboardLink.click();
    await expect(page).toHaveURL(/\/dashboard\/content$/);
  });
});
