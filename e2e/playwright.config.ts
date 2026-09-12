import { defineConfig, devices } from '@playwright/test';

export const FRONTEND_URL = 'http://localhost:5173';
export const BACKEND_URL = 'http://localhost:8080';

export default defineConfig({
  testDir: './tests',
  globalSetup: './global-setup.ts',
  // The suite shares one Postgres database and mutates rows; run serially.
  workers: 1,
  fullyParallel: false,
  retries: 0,
  timeout: 30_000,
  reporter: [['list']],
  use: {
    baseURL: FRONTEND_URL,
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
