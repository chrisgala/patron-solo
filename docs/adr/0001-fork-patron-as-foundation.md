# 0001 — Fork patroninc/patron as the foundation

Date: 2026-09-12 · Status: accepted

## Context

We need a single-creator membership site (subs, one-off content sales, posts/articles, PWA; livestreams later). Options: build from scratch in a stack of our choosing, or fork the open-source patron project (Rust/Actix + Diesel/Postgres + Redis + S3, React 19 Vite SSR client).

## Decision

Fork patron. It supplies working auth (email + verification, Google OAuth, Argon2, Redis sessions), Series→Posts CRUD with S3 file storage, and an SSR React client — roughly the bottom half of the product.

## Consequences

- All money features (tiers, Stripe, entitlements) are built fresh — patron has zero payment code and its old monetization columns were deliberately removed upstream.
- We inherit a Rust backend; backend changes require Rust/Diesel fluency.
- We inherit patron's OpenAPI→Speakeasy SDK pipeline; new endpoints use a hand-typed fetch wrapper until an SDK regen (see plan).
- Upstream remains a remote (`upstream`) for selective merges, but divergence is expected and accepted.
