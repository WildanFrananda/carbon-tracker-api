ALTER TABLE users 
ADD COLUMN refresh_token_hash VARCHAR(255),
ADD COLUMN refresh_token_expires_at TIMESTAMPTZ;

CREATE INDEX idx_users_refresh_token ON users(refresh_token_hash);