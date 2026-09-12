CREATE TABLE
  push_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    -- NULL for anonymous visitors who enabled notifications
    user_id UUID REFERENCES users (id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
