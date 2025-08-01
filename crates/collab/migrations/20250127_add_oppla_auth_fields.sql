-- Add Oppla authentication fields to users table
ALTER TABLE users ADD COLUMN oppla_user_id TEXT;
ALTER TABLE users ADD COLUMN oppla_account_id TEXT;
ALTER TABLE users ADD COLUMN username TEXT;

-- Create indexes for efficient lookup
CREATE INDEX idx_users_oppla_user_id ON users (oppla_user_id);
CREATE INDEX idx_users_oppla_account_id ON users (oppla_account_id);

-- Update the users table to allow null github_user_id (for Oppla-only users)
ALTER TABLE users ALTER COLUMN github_user_id DROP NOT NULL;
ALTER TABLE users ALTER COLUMN github_login DROP NOT NULL;

-- Add a constraint to ensure either github_user_id or oppla_user_id is present
ALTER TABLE users ADD CONSTRAINT check_auth_provider 
    CHECK (github_user_id IS NOT NULL OR oppla_user_id IS NOT NULL);