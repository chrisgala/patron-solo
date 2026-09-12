ALTER TABLE users
ADD COLUMN role VARCHAR(20) NOT NULL DEFAULT 'fan';

-- Single-creator site: at most one row may hold the creator role
CREATE UNIQUE INDEX one_creator_only ON users (role)
WHERE
  role = 'creator';
