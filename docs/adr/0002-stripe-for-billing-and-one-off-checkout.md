# 0002 — Stripe for subscriptions and one-off checkout

Date: 2026-09-12 · Status: accepted

## Context

The site sells recurring tier memberships and one-time access to individual Posts/Series. Single creator → payouts to one person, no marketplace/Connect requirements. Alternatives considered: Lemon Squeezy / Paddle (merchant-of-record, handles global VAT, higher fees, less flexible APIs), crypto.

## Decision

Stripe: Billing (Checkout `mode=subscription`) for Tiers, Checkout `mode=payment` for one-off Purchases, Billing Portal for self-service cancel/upgrade. Webhooks are the source of truth for subscription state, mirrored into a local `subscriptions` table; `stripe_events` provides idempotency.

## Consequences

- Tax compliance (VAT/sales tax) is the creator's problem (Stripe Tax can be enabled later); a merchant-of-record would have absorbed this.
- Local state is a mirror: every access decision reads our tables, never Stripe synchronously; webhook ordering races must be handled idempotently.
- Dev/test loop requires `stripe listen` CLI forwarding.
