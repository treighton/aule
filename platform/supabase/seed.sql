-- =============================================================================
-- Seed data for local development
-- Runs automatically on `supabase db reset`
-- =============================================================================

-- Test publisher (matches the GitHub username for namespace ownership)
INSERT INTO publishers (id, github_username, github_id, display_name, avatar_url, bio)
VALUES (
  '00000000-0000-0000-0000-000000000001',
  'treightonmauldin',
  1,
  'Treighton Mauldin',
  'https://github.com/treightonmauldin.png',
  'Building the open skill ecosystem'
)
ON CONFLICT (id) DO NOTHING;

-- API token for local testing
-- Raw token: test-token-123
-- Use with: Authorization: Bearer test-token-123
INSERT INTO api_tokens (publisher_id, token_hash, name)
VALUES (
  '00000000-0000-0000-0000-000000000001',
  encode(sha256(convert_to('test-token-123', 'UTF8')), 'hex'),
  'local-dev-token'
)
ON CONFLICT (token_hash) DO NOTHING;
