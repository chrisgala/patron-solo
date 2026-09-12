CREATE TABLE
  stripe_customers (
    user_id UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    stripe_customer_id TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

CREATE TABLE
  subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    tier_id UUID REFERENCES tiers (id) ON DELETE SET NULL,
    -- gating compares against this snapshot so tier deletion never breaks access
    tier_level INT NOT NULL,
    stripe_subscription_id TEXT NOT NULL UNIQUE,
    -- mirror of Stripe's status: active, trialing, past_due, canceled, ...
    status VARCHAR(30) NOT NULL,
    current_period_end TIMESTAMP,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

CREATE INDEX subscriptions_user_status ON subscriptions (user_id, status);
