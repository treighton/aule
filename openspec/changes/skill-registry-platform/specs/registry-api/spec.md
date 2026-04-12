## Overview

REST API for the skill registry, deployed as Next.js API routes on Vercel. Consumed by the web UI, the Rust CLI, and potentially third-party tools. All endpoints return JSON.

## Requirements

### Search API

**`GET /api/v1/search`**

Query parameters:
- `q` (string, required) — search query, matched against the FTS search_vector
- `runtime` (string, optional) — filter by adapter target (e.g., `claude-code`, `codex`)
- `tags` (string[], optional) — filter by tags (AND logic — skill must have all specified tags)
- `publisher` (string, optional) — filter by publisher github_username
- `limit` (integer, optional, default 20, max 100)
- `offset` (integer, optional, default 0)

Response:
```json
{
  "results": [
    {
      "registry_name": "@owner/skill-name",
      "name": "skill-name",
      "description": "...",
      "publisher": {
        "github_username": "owner",
        "display_name": "...",
        "avatar_url": "..."
      },
      "version": "1.0.0",
      "tags": ["testing", "code-review"],
      "adapter_targets": ["claude-code", "codex"],
      "verification_summary": {
        "status": "pass",
        "checks_passed": 5,
        "checks_warned": 0,
        "checks_failed": 0
      },
      "repo_url": "https://github.com/owner/repo",
      "last_indexed_at": "2025-01-15T10:30:00Z",
      "created_at": "2025-01-10T08:00:00Z"
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0
}
```

Search ranking: Postgres `ts_rank_cd` over the weighted search_vector. Results are ordered by rank descending.

### Skill Detail API

**`GET /api/v1/skills/@{owner}/{name}`**

Returns the full skill record with its latest version details.

Response:
```json
{
  "registry_name": "@owner/skill-name",
  "name": "skill-name",
  "description": "...",
  "publisher": {
    "github_username": "owner",
    "display_name": "...",
    "avatar_url": "...",
    "bio": "..."
  },
  "latest_version": {
    "version": "1.0.0",
    "manifest": { ... },
    "permissions": ["filesystem.read", "network.external"],
    "adapter_targets": ["claude-code", "codex"],
    "commit_sha": "abc123",
    "created_at": "2025-01-15T10:30:00Z"
  },
  "tags": ["testing"],
  "license": "MIT",
  "repo_url": "https://github.com/owner/repo",
  "skill_path": "skills/my-skill",
  "verification": {
    "status": "pass",
    "checks": [
      { "name": "manifest_schema", "status": "pass" },
      { "name": "contract_schema", "status": "pass" },
      { "name": "permission_vocabulary", "status": "pass" },
      { "name": "content_files", "status": "pass" },
      { "name": "description_present", "status": "pass" }
    ]
  },
  "last_indexed_at": "2025-01-15T10:30:00Z",
  "created_at": "2025-01-10T08:00:00Z"
}
```

**`GET /api/v1/skills/@{owner}/{name}/versions`**

Returns version history for a skill.

Response:
```json
{
  "versions": [
    {
      "version": "1.0.0",
      "is_latest": true,
      "commit_sha": "abc123",
      "adapter_targets": ["claude-code", "codex"],
      "created_at": "2025-01-15T10:30:00Z"
    },
    {
      "version": "0.9.0",
      "is_latest": false,
      "commit_sha": "def456",
      "adapter_targets": ["claude-code"],
      "created_at": "2025-01-05T08:00:00Z"
    }
  ]
}
```

### Resolution API

**`POST /api/v1/resolve`**

Used by the CLI to resolve a skill identifier into install coordinates.

Request:
```json
{
  "skill": "@owner/skill-name",
  "version_constraint": ">=1.0.0",
  "runtime": "claude-code"
}
```

`version_constraint` is optional (defaults to latest). `runtime` is optional (no adapter filtering if omitted).

Response:
```json
{
  "resolved": {
    "registry_name": "@owner/skill-name",
    "version": "1.0.0",
    "repo_url": "https://github.com/owner/repo",
    "ref": "main",
    "skill_path": "skills/my-skill",
    "commit_sha": "abc123",
    "manifest_hash": "sha256:...",
    "adapter_targets": ["claude-code", "codex"],
    "permissions": ["filesystem.read"],
    "verification_status": "pass"
  }
}
```

Error responses:
- 404: skill not found
- 422: no version matching constraint, or no compatible adapter for requested runtime

### Publisher API

**`GET /api/v1/publishers/{username}`**

Public profile for a publisher.

**`GET /api/v1/publishers/{username}/skills`**

List all skills by a publisher. Supports `limit` and `offset` query params.

### Skill Registration API (authenticated)

**`POST /api/v1/skills/register`**

Registers a new skill with the registry. Triggers immediate indexing.

Request:
```json
{
  "repo_url": "https://github.com/owner/repo",
  "skill_path": "skills/my-skill",
  "ref": "main"
}
```

- `skill_path` defaults to `"."` (skill.yaml is at repo root)
- `ref` defaults to `"main"`
- Server verifies namespace ownership (repo owner matches publisher's GitHub username or authorized org)
- Server immediately runs the indexing pipeline
- Returns the created skill record (same shape as skill detail) or validation errors

**`POST /api/v1/skills/@{owner}/{name}/refresh`**

Triggers re-indexing for an existing skill. Authenticated — must be the skill's publisher.

**`DELETE /api/v1/skills/@{owner}/{name}`**

Removes a skill from the registry. Authenticated — must be the skill's publisher. Deletes the skill and all versions/verification records. Does not affect the git repo.

### Error Response Format

All error responses follow a consistent format:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Skill @owner/name not found"
  }
}
```

Standard error codes: `NOT_FOUND`, `UNAUTHORIZED`, `FORBIDDEN`, `VALIDATION_ERROR`, `RATE_LIMITED`, `INTERNAL_ERROR`.

Validation errors include details:
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Skill registration failed",
    "details": [
      { "field": "repo_url", "message": "Repository not accessible" },
      { "field": "skill_path", "message": "No skill.yaml found at this path" }
    ]
  }
}
```

### Rate Limiting

- Public read endpoints: 100 requests/minute per IP
- Authenticated write endpoints: 30 requests/minute per publisher
- Rate limit headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

Rate limiting can be implemented via Vercel's built-in rate limiting or a simple Postgres-based counter.

### CORS

- API routes allow CORS from any origin for public read endpoints (enables third-party tools)
- Authenticated endpoints require same-origin or explicit CORS with credentials

## Acceptance Criteria

- Search returns ranked results matching keyword queries with correct FTS ranking
- Runtime and tag filters correctly narrow search results
- Skill detail returns full metadata including latest version and verification status
- Resolution endpoint returns install coordinates for a valid skill
- Resolution rejects version constraints that can't be satisfied
- Registration validates namespace ownership before creating the skill
- Registration triggers indexing and returns the indexed skill or validation errors
- Refresh triggers re-indexing and returns updated skill data
- Delete removes all skill data from the registry
- Error responses follow the consistent format with appropriate HTTP status codes
- Rate limiting prevents abuse without blocking normal usage
