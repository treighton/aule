## 1. Project Setup

- [ ] 1.1 Initialize Next.js app in `platform/` with App Router, TypeScript, Tailwind CSS
- [ ] 1.2 Initialize Supabase project and link to local dev (`supabase init`, `supabase link`)
- [ ] 1.3 Configure Vercel project with `platform/` as root directory
- [ ] 1.4 Add shared dependencies: Supabase client (`@supabase/supabase-js`), YAML parser, JSON Schema validator (`ajv`)
- [ ] 1.5 Set up environment variables: `SUPABASE_URL`, `SUPABASE_ANON_KEY`, `SUPABASE_SERVICE_ROLE_KEY`, `GITHUB_TOKEN`, `GITHUB_WEBHOOK_SECRET`
- [ ] 1.6 Verify local dev server starts and deploys to Vercel preview

## 2. Database Schema (registry-data-model)

- [ ] 2.1 Write Supabase migration for `publishers` table with constraints and indexes
- [ ] 2.2 Write Supabase migration for `skills` table with FTS search_vector, GIN indexes, and constraints
- [ ] 2.3 Write Supabase migration for `skill_versions` table with unique constraints and is_latest partial index
- [ ] 2.4 Write Supabase migration for `verification_results` table
- [ ] 2.5 Write Supabase migration for `device_auth_codes` and `api_tokens` tables
- [ ] 2.6 Write RLS policies for all tables (public read, authenticated write with ownership checks)
- [ ] 2.7 Write the search_vector update function as a Postgres function callable from the indexer
- [ ] 2.8 Verify migrations apply cleanly to local Supabase and remote

## 3. TypeScript Manifest Parser (skill-indexing)

- [ ] 3.1 Implement YAML manifest parser in `platform/lib/manifest.ts` — parse skill.yaml into typed TS object
- [ ] 3.2 Implement JSON Schema validation using the same `manifest.schema.json` from `crates/aule-schema/`
- [ ] 3.3 Implement contract parser in `platform/lib/contract.ts` — parse inline or referenced contracts
- [ ] 3.4 Implement permission vocabulary validation — check strings against known vocabulary, warn on unknown
- [ ] 3.5 Write tests using the same test fixtures as the Rust crate to verify parsing equivalence

## 4. GitHub API Client (skill-indexing)

- [ ] 4.1 Implement `platform/lib/github.ts` — fetchFileContent, fetchRepoMetadata, fetchLatestCommitSha, fileExists
- [ ] 4.2 Handle GitHub API error cases: 404 (not found), 403/429 (rate limited), network errors
- [ ] 4.3 Write tests with mocked GitHub API responses

## 5. Skill Indexing Pipeline (skill-indexing)

- [ ] 5.1 Implement `platform/lib/indexer.ts` — orchestrates: fetch from GitHub → parse → validate → upsert → verify
- [ ] 5.2 Implement change detection: compare commit SHA and manifest hash, skip if unchanged
- [ ] 5.3 Implement version management: insert new versions, manage is_latest flag
- [ ] 5.4 Implement verification result storage: upsert check results per skill version
- [ ] 5.5 Implement search_vector update after successful indexing
- [ ] 5.6 Write integration tests: index a real skill from a GitHub repo, verify database state

## 6. Publisher Authentication (publisher-auth)

- [ ] 6.1 Configure Supabase Auth with GitHub OAuth provider
- [ ] 6.2 Implement publisher profile creation/update on auth — database trigger or post-auth hook that upserts `publishers` row
- [ ] 6.3 Implement web login flow: `/auth/login` page, GitHub OAuth redirect, `/auth/callback` handler
- [ ] 6.4 Implement device auth start: `POST /api/v1/auth/device/start` — generate codes, store in DB, return to CLI
- [ ] 6.5 Implement device auth page: `/auth/device` — show user_code, "Authorize" button, trigger OAuth, update device_auth_codes on success
- [ ] 6.6 Implement device auth poll: `POST /api/v1/auth/device/poll` — check status, return token if completed
- [ ] 6.7 Implement API token generation and hash storage in `api_tokens` table
- [ ] 6.8 Implement API token auth middleware: extract Bearer token, hash, lookup, attach publisher to request
- [ ] 6.9 Implement namespace ownership verification: check GitHub org membership via GitHub API
- [ ] 6.10 Write tests for: device flow end-to-end, token auth middleware, namespace ownership check

## 7. Registry API Routes (registry-api)

- [ ] 7.1 Implement `GET /api/v1/search` — FTS query with runtime/tag/publisher filters, pagination, ranking
- [ ] 7.2 Implement `GET /api/v1/skills/@{owner}/{name}` — skill detail with latest version and verification
- [ ] 7.3 Implement `GET /api/v1/skills/@{owner}/{name}/versions` — version history
- [ ] 7.4 Implement `POST /api/v1/resolve` — resolve skill to install coordinates with version constraint matching
- [ ] 7.5 Implement `POST /api/v1/skills/register` — validate ownership, create skill record, trigger indexing
- [ ] 7.6 Implement `POST /api/v1/skills/@{owner}/{name}/refresh` — trigger re-indexing for existing skill
- [ ] 7.7 Implement `DELETE /api/v1/skills/@{owner}/{name}` — delete skill and cascade
- [ ] 7.8 Implement `GET /api/v1/publishers/{username}` and `GET /api/v1/publishers/{username}/skills`
- [ ] 7.9 Implement consistent error response format across all routes
- [ ] 7.10 Implement rate limiting (Postgres-based counter or Vercel built-in)
- [ ] 7.11 Write API integration tests for each endpoint

## 8. GitHub Webhook Handler (skill-indexing)

- [ ] 8.1 Implement `POST /api/webhooks/github` — verify signature, parse push event, match to registered skills
- [ ] 8.2 Trigger re-indexing for matched skills on push events
- [ ] 8.3 Write tests with sample webhook payloads and signature verification

## 9. Registry Web Application (registry-web)

- [ ] 9.1 Set up shadcn/ui with Tailwind CSS and dark mode support
- [ ] 9.2 Build landing page (`/`) with hero section, search bar, and featured skills
- [ ] 9.3 Build skill browse/search results page (`/skills`) with filters, skill cards, and pagination
- [ ] 9.4 Build skill detail page (`/skills/[owner]/[name]`) with full metadata, install command, contract, verification
- [ ] 9.5 Build publisher profile page (`/publishers/[name]`) with avatar, bio, and skill list
- [ ] 9.6 Build auth pages: login, callback, device authorization
- [ ] 9.7 Build publisher dashboard (`/dashboard`) with skill management (register, refresh, delete)
- [ ] 9.8 Add SEO: page titles, meta descriptions, Open Graph tags, sitemap
- [ ] 9.9 Verify responsive layout on mobile and desktop
- [ ] 9.10 Verify accessibility: keyboard navigation, ARIA labels, color contrast

## 10. CLI Extensions (cli-registry)

- [ ] 10.1 Add HTTP client dependency (`reqwest` or `ureq`) and `open` crate to `aule-cli/Cargo.toml`
- [ ] 10.2 Implement registry client module: HTTP requests with auth token, error handling, rate limit retries
- [ ] 10.3 Implement `skill login` — device auth flow with browser open and polling
- [ ] 10.4 Implement `skill logout` — remove token from config
- [ ] 10.5 Implement `skill publish` — detect git remote/ref/path, validate locally, register with API
- [ ] 10.6 Implement `skill search` — query API, format results as table, support --json
- [ ] 10.7 Implement `skill install @owner/name` — resolve via API, sparse git clone, local install + activate
- [ ] 10.8 Extend config.json schema in `aule-cache` to store registry_url, auth_token, publisher info
- [ ] 10.9 Write CLI integration tests for each new subcommand (using mock HTTP server or recorded responses)

## 11. End-to-End Validation

- [ ] 11.1 Publish one of the OpenSpec skills from this repo to the registry via `skill publish`
- [ ] 11.2 Verify it appears in search results via `skill search` and the web UI
- [ ] 11.3 Install it from the registry on a clean machine via `skill install @owner/name`
- [ ] 11.4 Verify the installed skill matches the local source (adapter output is identical)
- [ ] 11.5 Trigger a GitHub webhook push and verify re-indexing updates the registry
- [ ] 11.6 Publish all 4 OpenSpec skills and verify the full browse/search experience
