import { test, expect } from '@playwright/test';
import { uniqueEmail, query, makeVerifiedUser, cleanupE2eRows, TestUser } from '../helpers/db';
import { waitForMessage, extractVerificationLink } from '../helpers/mailpit';
import { loginViaUi } from '../helpers/auth';

let existingUser: TestUser;

test.describe('auth flows', () => {
  test.beforeAll(async () => {
    existingUser = await makeVerifiedUser('auth-existing');
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
  });

  test('register via UI -> verification email -> verify -> login via UI -> me -> logout', async ({
    page,
  }) => {
    const email = uniqueEmail('auth-reg');
    const password = 'E2ePassw0rd!';

    // Registration starts on /login: an unknown email routes to /register.
    await page.goto('/login');
    await page.getByPlaceholder('Enter your email').fill(email);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await expect(page).toHaveURL(/\/register$/);
    await expect(page.getByRole('heading', { name: 'Create your account' })).toBeVisible();

    await page.getByPlaceholder('Enter your display name').fill('e2e auth reg');
    await page.getByPlaceholder('Enter your password').fill(password);
    await page.getByPlaceholder('Repeat your password').fill(password);
    await page.getByRole('button', { name: 'Create Account' }).click();
    await expect(page).toHaveURL('http://localhost:5173/');

    // Verification mail lands in Mailpit with a token link
    const message = await waitForMessage(email, { subjectContains: 'verify' });
    const link = await extractVerificationLink(message.ID);
    // The backend redirects to the frontend with ?verified=success
    const verifyRes = await page.request.get(link, { maxRedirects: 0 });
    expect(verifyRes.status()).toBe(302);
    expect(verifyRes.headers()['location']).toContain('verified=success');

    const rows = await query<{ email_verified: boolean }>(
      'SELECT email_verified FROM users WHERE email = $1',
      [email],
    );
    expect(rows[0].email_verified).toBe(true);

    // Fresh session: log in through the UI
    await page.context().clearCookies();
    await loginViaUi(page, email, password);

    const me = await page.request.get('/proxy/api/auth/me');
    expect(me.ok()).toBeTruthy();
    const meBody = await me.json();
    expect(meBody.email).toBe(email);
    expect(meBody.role).toBe('fan');

    // Logout kills the session
    const logout = await page.request.get('/proxy/api/auth/logout');
    expect(logout.ok()).toBeTruthy();
    const meAfter = await page.request.get('/proxy/api/auth/me');
    expect(meAfter.status()).toBe(401);
  });

  test('wrong password is rejected with an error message', async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your email').fill(existingUser.email);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByPlaceholder('Enter your password').fill('definitely-wrong-1!');
    await page.getByRole('button', { name: 'Log in', exact: true }).click();
    await expect(page.getByText('Password is incorrect')).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);
  });

  test('/login shows the login page for anonymous visitors', async ({ page }) => {
    await page.goto('/login');
    await expect(page.getByRole('heading', { name: 'Log in or Sign up' })).toBeVisible();
    await expect(page.getByPlaceholder('Enter your email')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Continue', exact: true })).toBeVisible();
  });
});
