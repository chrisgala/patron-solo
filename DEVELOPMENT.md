# Local development

Single-creator membership site (fork of [patroninc/patron](https://github.com/patroninc/patron)).
See `CONTEXT.md` for the domain glossary and `docs/adr/` for key decisions.

## Stack

- `backend/` — Rust Actix-web + Diesel/Postgres + Redis sessions + S3 media + Stripe billing + Web Push
- `clients/react-server/` — React 19 Vite SSR fan/creator app
- `docker-compose.yml` — Postgres (5432), Redis (6380), Mailpit (SMTP 1025, UI 8025), MinIO (9000, bucket `patron-local`)

## First-time setup

```bash
# 1. Infra
docker compose up -d postgres redis mailpit minio minio-init

# 2. Backend env
cp backend/.env.dist backend/.env
#    - set AUTH_SECRET_KEY to 64+ random chars (python3 -c "import secrets;print(secrets.token_hex(32))")
#    - set CREATOR_EMAIL to the email you'll register as the creator
#    - set STRIPE_SECRET_KEY (sk_test_...) and STRIPE_WEBHOOK_SECRET (from stripe listen)
#    - generate VAPID keys for web push: npx web-push generate-vapid-keys

# 3. Run the backend (runs migrations automatically)
cd backend && cargo run --bin server

# 4. Stripe webhooks (separate terminal; requires stripe CLI, test mode)
stripe listen --forward-to localhost:8080/api/webhooks/stripe
#    put the whsec_... it prints into backend/.env STRIPE_WEBHOOK_SECRET and restart the backend

# 5. Frontend
cd clients/react-server
cp .env.dist .env.local
yarn install
npm run dev:local     # http://localhost:5173
```

## The creator account

Register normally (email goes to Mailpit — http://localhost:8025 — click the verification link).
The account whose email equals `CREATOR_EMAIL` is promoted to `creator` automatically
(at registration, and reconciled at every backend boot). Everyone else is a fan.

## Money flows (Stripe test mode)

- Tiers: create in the dashboard settings → creates Stripe Product+Price; fans subscribe via Checkout (`4242 4242 4242 4242`).
- One-off sales: set a price on a post or series; fans buy via Checkout `mode=payment`.
- Webhooks mirror subscription/purchase state into Postgres (`subscriptions`, `purchases`); the `stripe_events` table makes processing idempotent.
- Access rule (see `backend/shared/src/services/entitlements.rs`): free/rolled-free OR active sub with tier level ≥ post's `min_tier_level` OR owns post OR owns its series.

## Gotchas

- Media streams through `/api/cdn/files/{id}` which enforces entitlements — a file referenced by a gated published post 403s for non-entitled users, and gated media is served with `Cache-Control: private`.
- Role changes take effect on next login (the session stores a user snapshot).
- Web push is payload-less: the service worker wakes and fetches the newest post to build the notification.
