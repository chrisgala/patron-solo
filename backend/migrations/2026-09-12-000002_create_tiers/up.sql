CREATE TABLE
  tiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    -- ordering used for min-tier gating: higher level includes lower
    level INT NOT NULL UNIQUE,
    price_cents INT NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'usd',
    stripe_product_id TEXT,
    stripe_price_id TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
