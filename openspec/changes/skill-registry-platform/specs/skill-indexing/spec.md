## Overview

Pipeline that fetches skill metadata from GitHub repositories, validates it against the protocol schemas, and upserts normalized data into the registry database. This is the "write path" of the registry — it turns a git repo into a searchable skill listing.

## Requirements

### Indexing Pipeline

The indexer takes a registered skill record (repo URL + path + ref) and produces updated database records. It runs:

1. **Fetch metadata from GitHub**
   - Fetch `skill.yaml` from `{repo_owner}/{repo_name}/contents/{skill_path}/skill.yaml?ref={ref}` via GitHub Contents API
   - Fetch `content/skill.md` from the same path for description extraction
   - Fetch repo metadata (description, stars, license, default branch) via Repos API
   - Fetch the latest commit SHA on the tracked ref

2. **Parse and validate**
   - Parse `skill.yaml` as YAML into a manifest object
   - Validate manifest against the manifest JSON Schema (same schema used by the Rust `aule-schema` crate)
   - If the manifest references a contract, validate the contract against the contract JSON Schema
   - Validate permission strings against the permission vocabulary
   - Check that declared content paths exist in the repo (via Contents API HEAD requests)

3. **Detect changes**
   - Compare the commit SHA against `skills.last_indexed_sha`
   - Compare the manifest hash (SHA-256 of raw `skill.yaml` bytes) against the latest `skill_versions.manifest_hash`
   - If both match, skip — no changes since last index

4. **Upsert skill record**
   - Update `skills` row: description, tags, license, homepage_url, last_indexed_at, last_indexed_sha, search_vector
   - If the manifest version is new (not in `skill_versions`): insert a new `skill_versions` row, set `is_latest = true`, set all other versions' `is_latest = false`
   - If the manifest version exists but content changed: update the existing `skill_versions` row

5. **Store verification results**
   - Insert/upsert `verification_results` for each check: `manifest_schema`, `contract_schema`, `permission_vocabulary`, `content_files`, `description_present`
   - Each result has status (`pass`, `warning`, `error`) and a message

6. **Return indexing result**
   - Success: new version indexed, existing version updated, or no changes
   - Partial: skill indexed with verification warnings
   - Failure: manifest fetch failed, parse failed, or critical validation error

### TypeScript Manifest Parser

A TypeScript implementation of manifest parsing that matches the Rust `aule-schema` crate's behavior:

- Parse YAML using a standard YAML library (e.g., `yaml` npm package)
- Validate against `manifest.schema.json` using a JSON Schema validator (e.g., `ajv`)
- Extract typed fields: name, version, description, tags, contract, adapters, permissions, dependencies
- The JSON Schema files from `crates/aule-schema/` are the shared source of truth

The TS parser does NOT need to replicate all Rust crate functionality — it only needs to parse, validate, and extract metadata for indexing. The full contract validation, permission hierarchy, and envelope handling remain Rust-only for now.

### GitHub API Client

A thin wrapper around GitHub's REST API:

- `fetchFileContent(owner, repo, path, ref)` → file content (base64 decoded) or null
- `fetchRepoMetadata(owner, repo)` → { description, stars, license, default_branch, ... }
- `fetchLatestCommitSha(owner, repo, ref)` → SHA string
- `fileExists(owner, repo, path, ref)` → boolean

Authentication: GitHub token stored as an environment variable (`GITHUB_TOKEN`). For v1, a single personal access token or GitHub App installation token is sufficient.

Error handling: distinguish between "file not found" (404 → skill has no content at that path), "rate limited" (403/429 → retry with backoff), and "repo not accessible" (404 on repo → skill should be flagged).

### Indexing Triggers

The indexer is invoked by:

1. **Publisher registration** — when a skill is first registered via the API, immediately index it
2. **Manual refresh** — when a publisher hits the refresh endpoint
3. **GitHub webhook** — when a push event matches a registered skill's repo + ref
4. **Future: scheduled re-index** — periodic cron to catch missed updates (not in v1)

### Webhook Handler

- Endpoint: `POST /api/webhooks/github`
- Verify the `X-Hub-Signature-256` header against a webhook secret
- Parse the push event payload: extract repo full_name, ref, and commits
- Look up registered skills matching that repo URL and ref
- For each match, trigger re-indexing
- Return 200 immediately; indexing can be synchronous (fast enough for webhook timeout) or use a background mechanism if needed

## Acceptance Criteria

- Indexer correctly fetches and parses a skill.yaml from a public GitHub repo
- Manifest validation catches schema violations and reports them as verification results
- Change detection skips re-indexing when nothing has changed
- New versions are correctly inserted with `is_latest` management
- The TS manifest parser produces equivalent validation results to the Rust parser for the same input
- GitHub API errors (404, rate limit, network) are handled gracefully with appropriate error types
- Webhook signature verification rejects invalid payloads
- Webhook triggers re-indexing only for repos/refs that match registered skills
