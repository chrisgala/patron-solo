ALTER TABLE series
-- NULL = series is not sellable as a bundle
ADD COLUMN price_cents INT,
ADD COLUMN stripe_price_id TEXT,
-- default tier gate applied to new posts in this series
ADD COLUMN min_tier_level INT,
-- the hidden default series that holds standalone "update" posts
ADD COLUMN is_feed BOOLEAN NOT NULL DEFAULT false;
