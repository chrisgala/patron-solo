ALTER TABLE posts
ADD COLUMN kind VARCHAR(20) NOT NULL DEFAULT 'article',
-- NULL = no tier gate; otherwise requires active sub with tier level >= this
ADD COLUMN min_tier_level INT,
-- NULL = not individually purchasable
ADD COLUMN price_cents INT,
ADD COLUMN stripe_price_id TEXT,
-- rolling paywall: gates expire and the post becomes free at this time
ADD COLUMN free_at TIMESTAMP,
-- image posts reference their gallery files here (audio/video use existing columns)
ADD COLUMN image_file_ids UUID[];
