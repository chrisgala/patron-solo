DROP INDEX IF EXISTS one_creator_only;

ALTER TABLE users
DROP COLUMN role;
