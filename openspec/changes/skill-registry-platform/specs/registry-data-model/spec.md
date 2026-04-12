## Overview

Postgres schema for the skill registry, hosted on Supabase. This is the foundation — every other capability reads from or writes to these tables.

## Requirements

### Tables

**publishers**
- `id` UUID, primary key (Supabase auth user ID)
- `github_username` text, unique, not null — the GitHub login name
- `github_id` bigint, unique, not null — GitHub's numeric user ID (stable even if username changes)
- `display_name` text — human-friendly name (from GitHub profile)
- `avatar_url` text — profile image URL
- `bio` text — short description
- `website_url` text — optional homepage
- `created_at` timestamptz, not null, default now()
- `updated_at` timestamptz, not null, default now()

**skills**
- `id` UUID, primary key, default gen_random_uuid()
- `publisher_id` UUID, not null, references publishers(id)
- `name` text, not null — the manifest `name` field (kebab-case)
- `registry_name` text, unique, not null — `@{github_username}/{name}`, computed and stored for fast lookup
- `repo_url` text, not null — full GitHub repo URL (e.g., `https://github.com/owner/repo`)
- `repo_owner` text, not null — GitHub owner (user or org)
- `repo_name` text, not null — GitHub repo name
- `skill_path` text, not null, default `'.'` — path within repo to the skill root (where `skill.yaml` lives)
- `ref` text, not null, default `'main'` — git ref to track (branch or tag)
- `description` text — extracted from manifest
- `tags` text[] — extracted from manifest
- `license` text — from repo metadata
- `homepage_url` text — from manifest or repo
- `discovery_source` text, not null, default `'submitted'` — one of: `submitted`, `crawled`, `imported`
- `last_indexed_at` timestamptz — when the skill was last successfully indexed
- `last_indexed_sha` text — commit SHA at last index
- `search_vector` tsvector — Postgres FTS vector, updated on index
- `created_at` timestamptz, not null, default now()
- `updated_at` timestamptz, not null, default now()
- Unique constraint on `(publisher_id, name)` — a publisher can't have two skills with the same name

**skill_versions**
- `id` UUID, primary key, default gen_random_uuid()
- `skill_id` UUID, not null, references skills(id) on delete cascade
- `version` text, not null — semver string from manifest
- `manifest_hash` text, not null — SHA-256 of the raw skill.yaml content
- `manifest_snapshot` jsonb, not null — full parsed manifest stored as JSON
- `contract_snapshot` jsonb — parsed contract (null if inline in manifest)
- `permissions` text[] — extracted permission strings for filtering
- `adapter_targets` text[] — extracted adapter target IDs (e.g., `['claude-code', 'codex']`)
- `content_hash` text — SHA-256 of the skill.md content
- `commit_sha` text, not null — git commit SHA this version was indexed from
- `is_latest` boolean, not null, default false — denormalized flag for the latest indexed version
- `created_at` timestamptz, not null, default now()
- Unique constraint on `(skill_id, version)` — no duplicate versions per skill

**verification_results**
- `id` UUID, primary key, default gen_random_uuid()
- `skill_version_id` UUID, not null, references skill_versions(id) on delete cascade
- `check_name` text, not null — e.g., `manifest_schema`, `contract_schema`, `permission_vocabulary`, `content_files`
- `status` text, not null — one of: `pass`, `warning`, `error`
- `message` text — human-readable detail
- `created_at` timestamptz, not null, default now()
- Unique constraint on `(skill_version_id, check_name)` — one result per check per version

**device_auth_codes**
- `id` UUID, primary key, default gen_random_uuid()
- `device_code` text, unique, not null — opaque code for CLI polling
- `user_code` text, unique, not null — short code displayed to user
- `api_token` text — set after successful auth
- `publisher_id` UUID, references publishers(id) — set after successful auth
- `status` text, not null, default `'pending'` — one of: `pending`, `completed`, `expired`
- `expires_at` timestamptz, not null — TTL for the auth flow
- `created_at` timestamptz, not null, default now()

**api_tokens**
- `id` UUID, primary key, default gen_random_uuid()
- `publisher_id` UUID, not null, references publishers(id) on delete cascade
- `token_hash` text, unique, not null — SHA-256 hash of the token (never store plaintext)
- `name` text, not null — human label (e.g., `cli-2024-01-15`)
- `last_used_at` timestamptz
- `expires_at` timestamptz — null means no expiry
- `created_at` timestamptz, not null, default now()

### Indexes

- `skills.search_vector` — GIN index for FTS
- `skills.registry_name` — unique btree (already from unique constraint)
- `skills.publisher_id` — btree for publisher lookups
- `skills.tags` — GIN index for array containment queries
- `skill_versions.skill_id` — btree
- `skill_versions.(skill_id, is_latest)` — partial index where `is_latest = true`
- `verification_results.skill_version_id` — btree
- `device_auth_codes.device_code` — unique btree
- `device_auth_codes.status` — partial index where `status = 'pending'`
- `api_tokens.token_hash` — unique btree

### Search Vector Update

The `search_vector` column on `skills` is updated whenever a skill is re-indexed:

```sql
search_vector =
  setweight(to_tsvector('english', coalesce(name, '')), 'A') ||
  setweight(to_tsvector('english', coalesce(description, '')), 'B') ||
  setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'B') ||
  setweight(to_tsvector('english', coalesce(
    (SELECT github_username FROM publishers WHERE id = publisher_id), ''
  )), 'C')
```

### Row-Level Security

- `publishers`: read access is public; write access requires `auth.uid() = id`
- `skills`: read access is public; write access requires `auth.uid() = publisher_id`
- `skill_versions`: read access is public; write access requires matching publisher
- `verification_results`: read access is public; write is service-role only (indexing pipeline)
- `device_auth_codes`: read/write is service-role only
- `api_tokens`: read requires `auth.uid() = publisher_id`; write is service-role only

### Cleanup

- `device_auth_codes` with `status = 'pending'` and `expires_at < now()` should be marked `expired` by a periodic cleanup (Supabase cron or Vercel cron)
- Expired device codes older than 24 hours can be deleted

## Acceptance Criteria

- All tables created via Supabase migrations with proper constraints and indexes
- RLS policies enforce access control
- FTS search over the skills table returns ranked results matching keyword queries
- A skill with multiple versions correctly tracks `is_latest` (only one true per skill)
- Verification results are queryable per skill version
- Device auth codes expire correctly
- API tokens are stored as hashes, never plaintext
