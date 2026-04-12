## ADDED Requirements

### Requirement: Metadata endpoint location convention
A skill with a domain-based identity SHALL be resolvable via a metadata endpoint at `https://{domain}/{path}/.well-known/skill.json`. The endpoint SHALL return a JSON document conforming to the metadata endpoint schema.

#### Scenario: Standard endpoint resolution
- **WHEN** a client resolves the identity `skills.acme.dev/workflow/explore`
- **THEN** the client SHALL request `https://skills.acme.dev/workflow/explore/.well-known/skill.json`

#### Scenario: Domain without HTTPS
- **WHEN** a client attempts resolution and the HTTPS request fails
- **THEN** the client SHALL NOT fall back to HTTP and SHALL return a resolution error

### Requirement: Metadata endpoint response schema
The metadata endpoint JSON document SHALL contain: `identity` (string, canonical skill identity), `name` (string), `repository` (URL string to source), `manifest` (URL string to manifest file or relative path within repository), `versions` (array of version descriptor objects), and `updatedAt` (ISO 8601 timestamp).

#### Scenario: Complete metadata document
- **WHEN** a metadata endpoint returns a document with all required fields
- **THEN** the resolver SHALL parse the document and extract version/manifest references for installation

#### Scenario: Missing required field
- **WHEN** a metadata endpoint returns a document missing the `manifest` field
- **THEN** the resolver SHALL return a resolution error identifying the missing field

### Requirement: Version descriptor format
Each entry in the `versions` array SHALL contain: `version` (semver string), `contractVersion` (semver string), `manifest` (URL or relative path to version-specific manifest, optional — defaults to top-level `manifest`), and `checksums` (object mapping algorithm names to hex digest strings, optional).

#### Scenario: Multiple versions listed
- **WHEN** a metadata endpoint lists `versions: [{ version: "1.0.0", contractVersion: "1.0.0" }, { version: "1.1.0", contractVersion: "1.0.0" }]`
- **THEN** the resolver SHALL consider both versions when resolving with version constraints

#### Scenario: Version with checksum
- **WHEN** a version descriptor includes `checksums: { "sha256": "abc123..." }`
- **THEN** the resolver SHALL use the checksum for artifact integrity verification after download

### Requirement: Caching semantics
The metadata endpoint response MAY include standard HTTP caching headers (`Cache-Control`, `ETag`). Clients SHALL respect `Cache-Control: max-age` for re-resolution timing. Clients MUST NOT cache metadata for more than 24 hours regardless of headers.

#### Scenario: Cache-Control header present
- **WHEN** a metadata endpoint responds with `Cache-Control: max-age=3600`
- **THEN** the client SHALL use the cached response for up to 3600 seconds before re-resolving

#### Scenario: No caching headers
- **WHEN** a metadata endpoint responds without caching headers
- **THEN** the client SHALL use a default cache duration of 1 hour

### Requirement: v0 local-only resolution bypass
In v0, when a skill has no `identity` field (local name only), the resolver SHALL skip metadata endpoint resolution entirely and resolve directly from local filesystem paths or explicit source URLs provided in the manifest.

#### Scenario: Local skill without identity
- **WHEN** a manifest has `name: "openspec-explore"` with no `identity` field
- **THEN** the resolver SHALL NOT attempt HTTP metadata endpoint resolution and SHALL resolve from local source

#### Scenario: Explicit source URL in manifest
- **WHEN** a manifest includes `source: "https://github.com/org/repo"` without an `identity` field
- **THEN** the resolver SHALL use the source URL directly for artifact fetching, bypassing metadata endpoint resolution
