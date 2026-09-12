# End-to-end tests (Playwright)

Standalone npm package — it does not touch `clients/react-server`'s dependencies.

## Prerequisites

The full local stack must be running (see `../DEVELOPMENT.md`):

- Frontend SSR dev server on **http://localhost:5173** (`npm run dev:local` in `clients/react-server`)
- Backend API on **http://localhost:8080** (`cargo run --bin server` in `backend`)
- Postgres on **localhost:5432** (`postgres`/`password`), Mailpit on **:8025/:1025**, MinIO on **:9000** (all via `docker compose up -d`)
- The seeded creator account `creator@example.com` / `Creator123!` (the account matching `CREATOR_EMAIL`)

Stripe does **not** need to be configured: billing checkouts are asserted to fail
gracefully, and entitlement state (subscriptions/purchases) is inserted directly
into Postgres exactly as the Stripe webhook would write it.

## Running

```bash
cd e2e
npm install
npx playwright install chromium   # first time only
npx playwright test               # the whole suite
npx playwright test tests/auth.spec.ts   # one file
```

The suite runs single-worker because it shares the stack's database.
Global setup fails fast with instructions if part of the stack is down.

## Conventions

- All fixture emails/slugs/names are prefixed `e2e-`; `cleanupE2eRows()` deletes
  only those rows — the shared tables are never truncated.
- Prefer creating fixtures through the real API (register, login, create
  tier/series/post as the creator); raw SQL is used only for what has no API
  (email verification, subscriptions/purchases, rolling `free_at`).
- API requests go through the frontend proxy (`/proxy/api/...`) so cookies and
  routing match what the browser does.
- Known product bugs are marked `test.fixme()` with a comment describing the
  bug and the intended behavior; search the suite for `fixme` to list them.
