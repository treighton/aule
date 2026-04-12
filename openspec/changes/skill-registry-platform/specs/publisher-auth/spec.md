## Overview

Authentication and identity for skill publishers, built on Supabase Auth with GitHub OAuth. Publishers authenticate via GitHub to prove they own their GitHub username/org namespace. The CLI authenticates via a device authorization flow.

## Requirements

### GitHub OAuth via Supabase Auth

- Configure Supabase Auth with GitHub as an OAuth provider
- On first login, create a `publishers` row linked to the Supabase auth user:
  - `id` = Supabase auth user UUID
  - `github_username` = GitHub login
  - `github_id` = GitHub numeric ID
  - `display_name`, `avatar_url`, `bio` = from GitHub profile
- On subsequent logins, update profile fields if they've changed on GitHub
- Publisher can only register skills under their own GitHub username or organizations they have admin access to

### Namespace Ownership

A publisher can register a skill from repo `github.com/{owner}/{repo}` only if:
- `{owner}` matches their `github_username`, OR
- `{owner}` is a GitHub organization where they have `admin` or `maintain` role (verified via GitHub API at registration time)

This prevents a publisher from claiming skills in someone else's namespace.

### Web Authentication

Standard Supabase Auth flow for the web UI:
- "Sign in with GitHub" button
- Redirects to GitHub OAuth
- Returns to callback URL, Supabase creates/updates session
- Session stored as HTTP-only cookie
- Protected pages (publish, manage skills) check session

### CLI Device Authorization Flow

The CLI needs to authenticate without a browser redirect callback. The flow:

1. **CLI starts auth**: `POST /api/v1/auth/device/start`
   - Server generates `device_code` (opaque, 40 chars) and `user_code` (short, 8 chars, human-readable)
   - Stores in `device_auth_codes` table with 15-minute TTL
   - Returns: `{ device_code, user_code, verification_url, expires_in, interval }`

2. **CLI opens browser**: Opens `{verification_url}?code={user_code}` in the user's default browser
   - Fallback: prints the URL and user_code for manual entry

3. **User authenticates in browser**:
   - Registry page shows "Authorize CLI" with the user_code displayed
   - User clicks "Authorize" → GitHub OAuth flow
   - On successful auth, server updates the `device_auth_codes` row: sets `status = 'completed'`, sets `publisher_id`, generates and stores `api_token`
   - Also creates a row in `api_tokens` with the token hash

4. **CLI polls for completion**: `POST /api/v1/auth/device/poll` with `{ device_code }`
   - If pending: `{ status: "pending" }` (CLI waits `interval` seconds and retries)
   - If completed: `{ status: "completed", api_token, publisher: { github_username, display_name } }`
   - If expired: `{ status: "expired" }` (CLI shows error)

5. **CLI stores token**: Saves `api_token` and registry URL to `~/.skills/config.json`

### API Token Authentication

- API routes that require auth accept `Authorization: Bearer {api_token}` header
- Server hashes the token with SHA-256 and looks up `api_tokens.token_hash`
- If found and not expired, the request is authenticated as the associated publisher
- `last_used_at` is updated on each use

### Token Management

- Publishers can view their active tokens on the web UI (name, created date, last used — never the token value)
- Publishers can revoke tokens from the web UI
- `skill login` generates a new token each time (previous tokens remain valid)
- `skill logout` deletes the token from `~/.skills/config.json` (does not revoke server-side — user can do that from web UI)

## Acceptance Criteria

- GitHub OAuth login creates a publisher profile with correct GitHub metadata
- Publisher profile updates when GitHub profile changes
- Namespace ownership check prevents registering skills under another user's namespace
- Device auth flow completes end-to-end: CLI starts → browser auth → CLI receives token
- Device codes expire after 15 minutes
- API token authentication works for protected endpoints
- Tokens are stored as SHA-256 hashes, never plaintext
- Token revocation immediately prevents further API access with that token
