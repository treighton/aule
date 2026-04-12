## Overview

Extend the existing Rust CLI (`skill` binary) with commands that interact with the hosted registry: login, publish, search, and remote install. This connects the local toolchain to the network layer.

## Requirements

### Configuration

The registry URL and auth token are stored in `~/.skills/config.json` (already managed by the cache crate):

```json
{
  "registry_url": "https://aule.dev",
  "auth_token": "sk_...",
  "publisher": {
    "github_username": "owner",
    "display_name": "Owner Name"
  }
}
```

Default `registry_url` is compiled into the binary. Can be overridden via `--registry` flag or `SKILL_REGISTRY_URL` env var.

### `skill login`

Authenticates the CLI with the registry via the device authorization flow.

```
$ skill login
Opening browser to authenticate...
If the browser doesn't open, visit: https://aule.dev/auth/device?code=ABCD-1234

Waiting for authorization... done!
Logged in as @treightonmauldin (Treighton Mauldin)
Token saved to ~/.skills/config.json
```

Implementation:
1. `POST /api/v1/auth/device/start` → get device_code, user_code, verification_url
2. Open browser to verification_url (use `open` crate or equivalent)
3. Print fallback URL + user_code
4. Poll `POST /api/v1/auth/device/poll` with device_code every `interval` seconds
5. On completion: save api_token, registry_url, and publisher info to config
6. On expiry: print error, exit 1

Flags:
- `--registry <url>` — override registry URL

### `skill logout`

Removes the auth token from local config.

```
$ skill logout
Logged out. Token removed from ~/.skills/config.json
```

Does not revoke the token server-side (user can do that from web UI).

### `skill publish`

Registers a skill with the registry. Run from within a skill directory (containing `skill.yaml`) or with an explicit path.

```
$ skill publish
Publishing skills/openspec-explore from github.com/treightonmauldin/aule...

Validating skill.yaml... ok
Registering with registry... ok
Indexing... ok

Published @treightonmauldin/openspec-explore v0.1.0
  https://aule.dev/skills/treightonmauldin/openspec-explore
```

Implementation:
1. Read `skill.yaml` from current directory or `--path` flag
2. Validate locally (using existing `aule-schema` validation)
3. Detect the git remote URL and current ref from the git repo
4. Determine the skill_path relative to the repo root
5. `POST /api/v1/skills/register` with repo_url, skill_path, ref
6. Report success with registry URL, or report validation errors from the server

Flags:
- `--path <dir>` — path to skill directory (default: current directory)
- `--ref <ref>` — git ref to track (default: detected from git, usually `main`)
- `--json` — output JSON

Error cases:
- Not in a git repo → error with message
- No git remote → error with message
- Remote is not GitHub → error with message (v1 only supports GitHub)
- Not authenticated → error suggesting `skill login`
- Namespace mismatch → error from server (publisher doesn't own the namespace)
- skill.yaml not found → error
- Validation failure → error with details

### `skill search`

Searches the registry for skills matching a query.

```
$ skill search "code review"
@acme/code-review       Review code for quality issues          v1.2.0  ✓
@bob/pr-reviewer        Automated PR review with feedback       v0.5.0  ⚠
@eve/lint-helper        Linting assistance                      v2.0.1  ✓

3 results found. Install with: skill install @owner/name
```

Implementation:
1. `GET /api/v1/search?q={query}&runtime={runtime}&limit={limit}`
2. Display results as a formatted table
3. Show verification status indicator: ✓ (all pass), ⚠ (warnings), ✗ (errors)

Flags:
- `--runtime <target>` — filter by adapter target (e.g., `claude-code`)
- `--limit <n>` — max results (default 20)
- `--json` — output full JSON response

### `skill install @owner/name`

Installs a skill from the registry. Extends the existing `skill install` command to handle `@`-prefixed identifiers.

```
$ skill install @treightonmauldin/openspec-explore
Resolving @treightonmauldin/openspec-explore...
  version: 0.1.0
  repo: github.com/treightonmauldin/aule
  path: skills/openspec-explore

Fetching from git... done
Validating... ok
Installing to ~/.skills/cache/artifacts/sha256-abc123/... done
Activating for claude-code... done

Installed @treightonmauldin/openspec-explore v0.1.0
```

Implementation:
1. Detect `@`-prefix → route to registry resolution
2. `POST /api/v1/resolve` with skill identifier and optional version constraint
3. Get back repo_url, ref, skill_path, commit_sha
4. `git clone --depth 1 --filter=blob:none --sparse {repo_url}` into a temp directory
5. Sparse checkout the skill_path
6. Read and validate skill.yaml locally
7. Use existing `aule-cache` to store artifact and update metadata
8. Use existing `aule-adapter` + `aule-cache` to activate for default or specified target(s)

Flags:
- `--version <constraint>` — semver constraint (e.g., `">=1.0.0"`)
- `--target <runtime>` — activate for specific runtime (default: all configured targets)
- `--json` — output JSON

If the identifier does NOT start with `@`, the existing local-path install behavior is preserved.

### Registry HTTP Client

A new module in `aule-cli` (or a new `aule-registry` crate) that handles HTTP communication with the registry:

- Uses `reqwest` (or `ureq` for simpler sync HTTP) as the HTTP client
- Reads auth token from config
- Sets `Authorization: Bearer {token}` for authenticated requests
- Sets `User-Agent: skill-cli/{version}`
- Handles standard error response format
- Handles rate limit responses (retry after delay)

### New Dependencies

- HTTP client: `reqwest` (with `blocking` feature for sync CLI) or `ureq`
- Browser opening: `open` crate
- URL parsing: `url` crate (likely already available via other deps)

## Acceptance Criteria

- `skill login` completes the device auth flow and stores a valid token
- `skill logout` removes the token from config
- `skill publish` registers a skill from a git repo and reports the registry URL
- `skill publish` detects git remote, ref, and skill path automatically
- `skill publish` fails gracefully when not in a git repo or when remote is not GitHub
- `skill search` returns formatted results matching the query
- `skill search --json` returns the full API response
- `skill install @owner/name` resolves via registry, clones from git, and installs/activates locally
- `skill install ./local-path` continues to work as before (no regression)
- All commands respect `--json` flag for machine-readable output
- All commands print helpful error messages for common failure cases
- Auth token is sent only to the configured registry URL, never to other hosts
