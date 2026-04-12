## Why

Phase 1 built the core protocol schemas and local toolchain — skills can be authored, validated, built, and activated locally. But there's no way to discover or share skills beyond copying directories. The ecosystem needs a public registry where skill authors can publish their work and users can find, evaluate, and install skills from a single search. Starting now because the skill/plugin ecosystem across coding agents is growing fast, and an open, searchable registry with git-backed publishing establishes the standard distribution model before proprietary alternatives emerge.

## What Changes

- Build a **hosted skill registry** (Next.js on Vercel + Supabase) that indexes skill metadata from GitHub repositories
- Define a **GitHub-scoped identity model** (`@owner/skill-name`) for registry-listed skills, forward-compatible with domain-based identity later
- Build a **skill indexing pipeline** that fetches `skill.yaml` from GitHub repos via the GitHub API, validates it, and stores normalized metadata in Postgres
- Build a **publisher authentication flow** using GitHub OAuth so skill authors can register and manage their listings
- Build a **registry REST API** for search, skill detail, resolution, and publishing — consumed by both the web UI and the CLI
- Build a **registry web application** with landing page, search/browse, skill detail pages, and publisher profiles
- Extend the **Rust CLI** with `skill publish`, `skill search`, `skill install @owner/name`, and `skill login` commands that interact with the registry API
- Design the **data model** to track discovery source (submitted vs future-crawled) to support future GitHub crawling

## Capabilities

### New Capabilities
- `registry-data-model`: Postgres schema for publishers, skills, skill versions, contracts, adapters, verification results, and discovery events — hosted on Supabase
- `skill-indexing`: Pipeline that fetches skill.yaml + content metadata from GitHub repos via GitHub Contents API, validates against the manifest/contract schemas, and upserts normalized metadata into the registry database
- `publisher-auth`: GitHub OAuth authentication flow for publisher identity, profile management, and API key issuance for CLI access
- `registry-api`: REST API exposing search (Postgres FTS), skill detail, version listing, resolution endpoints, and publisher submission/management — deployed as Next.js API routes on Vercel
- `registry-web`: Next.js App Router web application with landing/search page, skill browse/search, skill detail pages (metadata, contract, adapters, verification status), and publisher profile pages
- `cli-registry`: Rust CLI extensions — `skill login` (device auth flow), `skill publish` (register repo URL), `skill search` (query registry), `skill install @owner/name` (resolve via registry + clone from git)

### Modified Capabilities
- `resolver`: Extended to resolve skills from the registry API in addition to local path and cache — registry becomes a new resolution source in the source chain
- `cache-manager`: Extended to store registry-resolved metadata alongside locally-installed artifacts

## Impact

- New `platform/` directory in the monorepo containing the Next.js application, Supabase migrations, and shared TypeScript libraries
- New Supabase project providing Postgres database, authentication, and storage
- Vercel deployment for the Next.js app (API routes + web UI in a single deployment)
- Existing Rust CLI gains 4 new subcommands and a registry client module
- The resolver crate gains a registry resolution source that queries the API before falling back to local/cache
- Protocol schemas from Phase 1 (manifest, contract, permissions) are consumed server-side in TypeScript — the TS codebase needs its own manifest parser that matches the Rust implementation's behavior
- GitHub API dependency for indexing (authenticated, 5000 req/hr rate limit)
- Publisher identity is coupled to GitHub accounts in v1 — this is intentional and can be extended to other identity providers later
