## Context

Phase 1 delivered the core protocol: manifest schema, contract schema, permission vocabulary, invocation envelope, metadata endpoint spec, adapter generator, resolver, cache manager, and CLI. The 4 OpenSpec skills validate the full local flow — author a skill.yaml, build adapter output for Claude Code and Codex, install and activate locally.

What's missing is the network layer. Skills can only be shared by copying directories or cloning repos manually. There's no way to search for skills, no publisher identity, no centralized index.

This design covers the first-party registry platform — a hosted service that indexes skill metadata from GitHub repos and exposes it via API and web UI. The Rust CLI is extended to publish, search, and install from the registry.

Key constraint: solo developer, TypeScript for the platform (Next.js + Supabase), Rust for CLI extensions. Git (GitHub) is the source of truth for skill content — the registry indexes metadata, it doesn't host artifacts.

## Goals / Non-Goals

**Goals:**
- Host a public skill registry on Vercel + Supabase that indexes skills from GitHub repos
- Support publisher registration via GitHub OAuth
- Provide keyword search over indexed skills via Postgres full-text search
- Serve a web UI for browsing, searching, and inspecting skills
- Expose a REST API consumed by both the web UI and the Rust CLI
- Extend the CLI with publish, search, install-from-registry, and login commands
- Design the data model to accommodate future GitHub crawling and additional discovery sources
- Validate manifests and contracts at index time and surface verification status

**Non-Goals:**
- Semantic/vector search or AI-powered recommendations (v1 uses Postgres FTS only)
- Hosting skill artifacts — the registry stores metadata, git stores content
- Supporting git forges other than GitHub in v1 (GitLab, Bitbucket come later)
- Enterprise policy service or organization-scoped controls
- Telemetry ingestion from skill executions
- Billing, monetization, or marketplace mechanics
- Verification beyond manifest/contract schema validation (no behavioral evals)
- Domain-based identity resolution via `.well-known/skill.json` (forward-compatible but not active in v1)
- OAuth providers other than GitHub for publisher identity

## Decisions

### 1. Monorepo with `platform/` directory

**Decision:** The Next.js app lives in `platform/` within the existing aule repo. Supabase migrations live in `platform/supabase/migrations/`.

**Rationale:** The CLI and platform share the skill protocol — manifest schema, contract schema, permission vocabulary. When the protocol evolves, both sides need to update together. A monorepo keeps this atomic. The Next.js app deploys to Vercel independently via the root directory setting in the Vercel project config.

**Structure:**
```
aule/
├── crates/                    ← existing Rust workspace
├── platform/                  ← new Next.js app
│   ├── app/                   ← App Router pages + API routes
│   │   ├── page.tsx           ← landing + search
│   │   ├── skills/
│   │   │   └── [owner]/
│   │   │       └── [name]/
│   │   │           └── page.tsx
│   │   ├── publishers/
│   │   │   └── [name]/
│   │   │       └── page.tsx
│   │   └── api/
│   │       └── v1/
│   │           ├── search/
│   │           ├── skills/
│   │           ├── publishers/
│   │           └── auth/
│   ├── lib/                   ← shared TS libraries
│   │   ├── manifest.ts        ← manifest parser (mirrors Rust)
│   │   ├── contract.ts        ← contract parser
│   │   ├── github.ts          ← GitHub API client
│   │   ├── indexer.ts         ← skill indexing pipeline
│   │   └── db.ts              ← Supabase client + queries
│   ├── supabase/
│   │   ├── migrations/        ← Postgres DDL migrations
│   │   └── config.toml
│   ├── package.json
│   ├── tsconfig.json
│   └── next.config.ts
├── skills/                    ← existing skill sources
└── Cargo.toml                 ← existing Rust workspace
```

**Alternatives considered:**
- Separate repo: loses atomic protocol changes, adds coordination overhead for solo dev
- Turborepo monorepo with packages: premature — there's one app, not multiple

### 2. GitHub-scoped identity with forward-compatible domain identity

**Decision:** Registry skills are identified by `@owner/skill-name` where `owner` is the GitHub username or organization. The manifest's `name` field remains the skill's local name. The registry computes the full identity from the publisher's GitHub account + manifest name.

**Rationale:** GitHub already owns a collision-free namespace. For a git-centric registry, `@owner/name` is the natural identifier — it maps directly to where the skill lives. The npm ecosystem proved this model works at scale. Domain-based identity (`skills.acme.dev/name`) remains available via the manifest's optional `identity` field for future protocol-level resolution, but the registry doesn't require or use it in v1.

**Identity resolution:**
```
Registry identity:    @treightonmauldin/openspec-explore
Git location:         github.com/treightonmauldin/aule (repo) + skills/openspec-explore (path)
Manifest name:        openspec-explore
Protocol identity:    skills.aule.dev/openspec-explore (optional, v2+)
```

**Important:** A single repo can contain multiple skills. The publisher registers a repo + path, not just a repo.

**Alternatives considered:**
- Domain-based identity only: requires DNS setup, too much friction for v1
- Flat names with publisher metadata: collision risk, less intuitive

### 3. GitHub API for skill fetching (not git clone)

**Decision:** The indexing pipeline fetches skill metadata via GitHub's REST API (Contents API, Repos API), not by cloning repositories.

**Rationale:** The registry runs on Vercel serverless functions — git clones are impractical in that environment. The GitHub API provides direct file access (`GET /repos/{owner}/{repo}/contents/{path}`), repo metadata, and branch/tag information via HTTP. Rate limits (5000/hr authenticated) are more than sufficient for a registry that indexes on publisher submission + periodic refresh, not continuous crawling.

**What gets fetched:**
- `skill.yaml` (manifest) — via Contents API
- `content/skill.md` (skill body) — via Contents API, for description extraction only
- Repo metadata (stars, description, license) — via Repos API
- Latest commit SHA on the tracked ref — for change detection

**Alternatives considered:**
- Shallow git clone: requires git binary, doesn't work well in serverless
- GitHub Archive API (tarball): fetches everything, wasteful when we need 2-3 files
- Git clone in a background worker: adds infrastructure complexity (need a long-running process)

### 4. Supabase for Postgres + Auth, Vercel for compute + hosting

**Decision:** Supabase provides the Postgres database and GitHub OAuth integration. Vercel hosts the Next.js app (pages + API routes). No additional infrastructure.

**Rationale:** Supabase gives Postgres (with FTS, pg_trgm), GitHub OAuth, and row-level security in one managed service. Vercel gives serverless API routes and static/SSR page hosting. Together they cover all platform needs without managing servers. Solo dev constraint means minimizing operational surface.

**Auth flow:**
```
Publisher → Vercel (Next.js) → Supabase Auth (GitHub OAuth)
                                    │
                                    ▼
                              GitHub OAuth
                                    │
                                    ▼
                              Supabase creates user
                              linked to GitHub identity
```

CLI auth uses a device authorization flow: `skill login` opens a browser, user authenticates via GitHub OAuth on the registry web app, CLI receives an API token.

**Alternatives considered:**
- Self-managed Postgres: unnecessary operational burden
- Clerk/Auth0 for auth: adds another service; Supabase Auth handles GitHub OAuth natively
- Edge functions on Supabase: would work but Vercel is already hosting the web app, keep compute in one place

### 5. Postgres full-text search for v1

**Decision:** Search is implemented using Postgres `tsvector`/`tsquery` with `pg_trgm` for fuzzy matching. No separate search index.

**Rationale:** For a v1 registry with hundreds to low thousands of skills, Postgres FTS is more than adequate. It supports ranking, prefix matching, and combined filtering (by runtime, permissions, publisher). Adding OpenSearch/Elasticsearch would be premature infrastructure for the current scale. The data model includes a `search_vector` column on the skills table that's updated on every index operation.

**Search fields (weighted):**
- A: skill name (highest weight)
- B: description, tags
- C: publisher name
- D: permission names, adapter targets

**Alternatives considered:**
- Typesense/Meilisearch: good developer experience but another service to manage
- Supabase Vector (pgvector): semantic search is a v2+ feature
- Client-side search: doesn't scale, bad for SEO

### 6. Indexing as on-demand + webhook, not crawling

**Decision:** v1 indexing is triggered by: (1) publisher submission (`POST /api/v1/skills/register`), (2) manual refresh (`POST /api/v1/skills/{id}/refresh`), (3) optional GitHub webhook on push. No scheduled crawling.

**Rationale:** Publisher-submitted indexing is the simplest model — the publisher knows when their skill changes and can trigger re-indexing. GitHub webhooks provide near-real-time updates without polling. Scheduled crawling (checking all indexed repos periodically) can be added later as a cron job but isn't needed when publishers actively manage their listings.

**Webhook flow:**
```
GitHub push → webhook → POST /api/webhooks/github
  → verify signature
  → check if repo+path matches a registered skill
  → re-index if match
```

**Future crawling addition (v2+):**
A cron job queries GitHub Search API for repos containing `skill.yaml`, discovers unregistered skills, and creates "crawled" discovery records. Publisher can then claim ownership.

**Alternatives considered:**
- Periodic polling of all registered repos: wasteful, rate-limit unfriendly
- Only manual refresh: too much friction for active publishers

### 7. Verification as validation signals, not gates

**Decision:** At index time, the registry validates the manifest against the JSON Schema, validates contracts, checks permission vocabulary, and checks that declared content files exist in the repo. Results are stored as verification records and surfaced in the UI/API as signals (badges/status), not as publication gates.

**Rationale:** Following the architecture doc's guidance: "Results should be surfaced as signals rather than absolute publication gates in v1." A skill with validation warnings still appears in search results — it just shows reduced trust signals. This keeps publishing frictionless while still rewarding well-formed skills.

**Verification checks (v1):**
- Manifest schema compliance (error/warning/pass)
- Contract schema compliance (error/warning/pass)
- Permission vocabulary validation (unknown permissions = warning)
- Content file existence in repo (missing = error)
- README/description presence (missing = warning)

**Alternatives considered:**
- Hard gates (reject invalid skills): too restrictive for a young ecosystem
- No verification: loses a key value proposition of the registry
- Async verification workers: overkill for synchronous schema validation

### 8. REST API with versioned paths

**Decision:** All API endpoints live under `/api/v1/` as Next.js API routes. Responses are JSON. Authentication uses Bearer tokens (API keys for CLI, session cookies for web).

**API surface:**

```
Public (no auth required):
  GET  /api/v1/search?q=&runtime=&limit=&offset=
  GET  /api/v1/skills/@{owner}/{name}
  GET  /api/v1/skills/@{owner}/{name}/versions
  GET  /api/v1/skills/@{owner}/{name}/verification
  GET  /api/v1/publishers/{name}
  GET  /api/v1/publishers/{name}/skills
  POST /api/v1/resolve

Publisher (auth required):
  POST /api/v1/skills/register
  POST /api/v1/skills/@{owner}/{name}/refresh
  DELETE /api/v1/skills/@{owner}/{name}

Auth:
  POST /api/v1/auth/device/start
  POST /api/v1/auth/device/poll
  GET  /api/v1/auth/callback (GitHub OAuth redirect)
```

**Alternatives considered:**
- GraphQL: more flexible but adds complexity for a solo dev; REST is simpler to build, test, and consume from the Rust CLI
- tRPC: great for TS-to-TS but the primary API consumer is a Rust CLI

### 9. CLI auth via device flow

**Decision:** `skill login` initiates a device authorization flow: the CLI generates a code, opens the registry in a browser, the user authenticates via GitHub OAuth, and the CLI polls for token completion. The token is stored in `~/.skills/config.json`.

**Rationale:** Device flow is the standard pattern for CLI-to-web auth (GitHub CLI, Vercel CLI, Supabase CLI all use it). It works in headless environments and doesn't require the CLI to run a local HTTP server.

**Flow:**
```
CLI                          Registry                    GitHub
 │                              │                          │
 │ POST /auth/device/start      │                          │
 │─────────────────────────────▶│                          │
 │ { device_code, user_code,    │                          │
 │   verification_url }         │                          │
 │◀─────────────────────────────│                          │
 │                              │                          │
 │ Opens browser to             │                          │
 │ verification_url?code=...    │                          │
 │                              │                          │
 │                              │ OAuth redirect to GitHub │
 │                              │─────────────────────────▶│
 │                              │                          │
 │                              │◀── callback with token ──│
 │                              │                          │
 │ POST /auth/device/poll       │                          │
 │─────────────────────────────▶│                          │
 │ { api_token }                │                          │
 │◀─────────────────────────────│                          │
 │                              │                          │
 │ Stores token in              │                          │
 │ ~/.skills/config.json        │                          │
```

**Alternatives considered:**
- Local HTTP server callback: more complex, port conflicts, firewall issues
- Copy-paste token from web: works but worse UX
- GitHub device flow directly: would couple CLI auth to GitHub, not the registry

### 10. Extend resolver with registry source

**Decision:** The Rust resolver gains a `resolve_from_registry` source that queries `POST /api/v1/resolve` with a skill identifier and version constraint. The resolution chain becomes: cache → registry → local path → error.

**Rationale:** The resolver already supports multiple sources (local path, cache). Adding registry as a source follows the existing pattern. Registry resolution returns a git URL + ref + path, which the installer then fetches via git clone (the CLI can run git, unlike serverless functions).

**Install flow from registry:**
```
skill install @owner/name
  │
  ├── resolve_from_cache → miss
  ├── resolve_from_registry → hit
  │     │
  │     └── returns: { repo_url, ref, path, version, manifest_hash }
  │
  ├── git clone --depth 1 --filter=blob:none {repo_url}
  ├── read skill.yaml from {path}
  ├── validate locally
  ├── store in ~/.skills/cache/artifacts/{hash}/
  ├── update metadata index
  └── activate for target runtime(s)
```

**Alternatives considered:**
- Download tarball from GitHub API: works but git clone is more natural for the Rust CLI and supports private repos with git credentials
- Have the registry serve the manifest directly: adds artifact hosting responsibility to the registry, against the design principle

## Risks / Trade-offs

**[GitHub API coupling] → Mitigation:** The indexing pipeline is behind an abstraction (`GitProvider` interface) so adding GitLab/Bitbucket support later doesn't require restructuring. The GitHub-specific code is isolated in `lib/github.ts`.

**[GitHub rate limits during future crawling] → Mitigation:** v1 only indexes on publisher submission + webhooks, well within limits. Future crawling will need a GitHub App with higher rate limits or a queue-based approach with backoff. The data model tracks `discovery_source` to distinguish submitted vs crawled skills.

**[TypeScript manifest parser must match Rust behavior] → Mitigation:** Both implementations validate against the same JSON Schema files (`manifest.schema.json`, `contract.schema.json`) shipped in the repo. Integration tests in the TS codebase use the same test fixtures as the Rust tests. Schema files are the source of truth, not either implementation.

**[Single Postgres for everything] → Mitigation:** Accepted for v1. Postgres FTS handles the search load at this scale. If search needs outgrow Postgres, adding a dedicated search index is a straightforward migration — the search API is an abstraction over the query, not a direct Postgres dependency in the route handlers.

**[Device auth flow complexity] → Mitigation:** The device flow has well-established patterns. The registry stores pending device codes in a Postgres table with TTL. The CLI polls with exponential backoff. Libraries exist for both sides.

**[Monorepo complexity] → Mitigation:** The Rust and TS codebases are independent build targets. `cargo build` ignores `platform/`, `next build` ignores `crates/`. Vercel deploys from `platform/` root. The only shared artifacts are the JSON Schema files, which are read (not imported) by both sides.

**[Vercel serverless function limits] → Mitigation:** Skill indexing (fetching from GitHub API + validating + upserting) is well within the 300s function timeout. The heaviest operation is fetching 2-3 files from GitHub and running JSON Schema validation — typically under 2 seconds.

**[Solo dev maintaining two language ecosystems] → Mitigation:** The Rust CLI extensions are modest (HTTP client + JSON parsing, ~4 new subcommands). The bulk of new development is TypeScript, which is the faster iteration language. The Rust side is primarily a consumer of the API, not a complex system.
