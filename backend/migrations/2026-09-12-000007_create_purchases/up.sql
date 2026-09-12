CREATE TABLE
  purchases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- exactly one of post_id / series_id is set
    post_id UUID REFERENCES posts (id) ON DELETE CASCADE,
    series_id UUID REFERENCES series (id) ON DELETE CASCADE,
    stripe_checkout_session_id TEXT,
    stripe_payment_intent_id TEXT UNIQUE,
    amount_cents INT,
    status VARCHAR(20) NOT NULL DEFAULT 'paid',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
      (
        post_id IS NOT NULL
        AND series_id IS NULL
      )
      OR (
        post_id IS NULL
        AND series_id IS NOT NULL
      )
    )
  );

CREATE UNIQUE INDEX purchases_user_post ON purchases (user_id, post_id)
WHERE
  post_id IS NOT NULL;

CREATE UNIQUE INDEX purchases_user_series ON purchases (user_id, series_id)
WHERE
  series_id IS NOT NULL;
