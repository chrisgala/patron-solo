-- Processed Stripe webhook event ids, for idempotent webhook handling
CREATE TABLE
  stripe_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
