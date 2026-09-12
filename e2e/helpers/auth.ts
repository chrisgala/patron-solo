import { expect, request, APIRequestContext, Page } from '@playwright/test';
import { FRONTEND_URL } from '../playwright.config';

export const CREATOR = { email: 'creator@example.com', password: 'Creator123!' };

/**
 * Returns an APIRequestContext logged in as the given user. Requests go
 * through the frontend's /proxy so the same-origin session cookie is used,
 * matching what the browser does. Call `.dispose()` when done.
 */
export async function apiContextFor(email: string, password: string): Promise<APIRequestContext> {
  const ctx = await request.newContext({ baseURL: FRONTEND_URL });
  const res = await ctx.post('/proxy/api/auth/login', { data: { email, password } });
  if (!res.ok()) {
    throw new Error(`API login failed for ${email}: ${res.status()} ${await res.text()}`);
  }
  return ctx;
}

/** An anonymous APIRequestContext against the frontend proxy. */
export async function anonContext(): Promise<APIRequestContext> {
  return request.newContext({ baseURL: FRONTEND_URL });
}

/**
 * Logs the page's browser context in via the API (fast path: page.request
 * shares the cookie jar with the page).
 */
export async function loginViaApi(page: Page, email: string, password: string): Promise<void> {
  const res = await page.request.post('/proxy/api/auth/login', { data: { email, password } });
  if (!res.ok()) {
    throw new Error(`API login failed for ${email}: ${res.status()} ${await res.text()}`);
  }
}

/**
 * Logs in through the real two-step /login form (email -> password) and
 * waits for the post-login redirect.
 */
export async function loginViaUi(page: Page, email: string, password: string): Promise<void> {
  await page.goto('/login');
  await page.getByPlaceholder('Enter your email').fill(email);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page.getByPlaceholder('Enter your password').fill(password);
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  // Lands on /home which the router serves as the public home page,
  // or on / when ProtectedRoute redirects the now-authenticated user.
  await expect(page).toHaveURL(new RegExp(`${FRONTEND_URL}/(home)?$`));
}
