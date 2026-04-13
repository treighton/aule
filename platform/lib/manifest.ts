// Manifest types and parser matching crates/aule-schema/src/manifest.rs

import YAML from "yaml";
import { type Contract, parseContract } from "./contract";

// --- Types ---

export interface ContentPaths {
  skill: string;
  commands?: Record<string, string>;
}

export interface AdapterConfig {
  enabled: boolean;
  [key: string]: unknown;
}

export interface SkillDependency {
  name: string;
  version?: string;
}

export interface ToolDependency {
  name: string;
  version?: string;
}

export interface Dependencies {
  skills?: SkillDependency[];
  tools?: ToolDependency[];
}

export interface ManifestMetadata {
  author?: string;
  license?: string;
  homepage?: string;
  repository?: string;
  tags?: string[];
  [key: string]: unknown;
}

export type ContractRef = { kind: "inline"; value: Contract } | { kind: "file"; path: string };

export interface Manifest {
  schemaVersion: string;
  name: string;
  description: string;
  version: string;
  content: ContentPaths;
  contract: ContractRef;
  identity?: string;
  adapters: Record<string, AdapterConfig>;
  dependencies?: Dependencies;
  metadata?: ManifestMetadata;
  extensions?: Record<string, unknown>;
}

// --- v0.2.0 Types ---

export interface SkillDefinition {
  description: string;
  entrypoint: string;
  version: string;
  inputs?: unknown;
  outputs?: unknown;
  permissions?: string[];
  determinism?: "deterministic" | "bounded" | "probabilistic";
  errors?: Array<{ code: string; description: string }>;
  behavior?: {
    latencyClass?: "fast" | "moderate" | "slow";
    costClass?: "free" | "low" | "medium" | "high";
    sideEffects?: boolean;
  };
  commands?: Record<string, string>;
}

export interface Tool {
  description: string;
  using: string;
  version?: string;
  entrypoint: string;
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
}

export interface Hooks {
  onInstall?: string;
  onActivate?: string;
  onUninstall?: string;
}

export interface ManifestV2 {
  schemaVersion: string;
  name: string;
  description: string;
  version: string;
  files: string[];
  skills: Record<string, SkillDefinition>;
  tools?: Record<string, Tool>;
  hooks?: Hooks;
  identity?: string;
  adapters: Record<string, AdapterConfig>;
  dependencies?: Dependencies;
  metadata?: ManifestMetadata;
  extensions?: Record<string, unknown>;
}

export type ManifestAny =
  | { version: "0.1.0"; manifest: Manifest }
  | { version: "0.2.0"; manifest: ManifestV2 };

// --- Errors ---

export class ManifestParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ManifestParseError";
  }
}

export class ManifestValidationError extends Error {
  constructor(
    message: string,
    public readonly errors: string[]
  ) {
    super(message);
    this.name = "ManifestValidationError";
  }
}

// --- Validation result ---

export interface ManifestValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// --- Helpers ---

function isKebabCase(s: string): boolean {
  if (s.length === 0) return false;
  if (s.startsWith("-") || s.endsWith("-") || s.includes("--")) return false;
  return /^[a-z0-9-]+$/.test(s);
}

function isValidIdentity(s: string): boolean {
  const slashPos = s.indexOf("/");
  if (slashPos === -1) return false;
  const domain = s.slice(0, slashPos);
  const path = s.slice(slashPos + 1);
  return domain.includes(".") && domain.length > 0 && path.length > 0;
}

function isSemver(s: string): boolean {
  const parts = s.split(".");
  if (parts.length !== 3) return false;
  return parts.every((p) => /^\d+$/.test(p));
}

function parseContractRef(raw: unknown): ContractRef {
  if (typeof raw === "string") {
    return { kind: "file", path: raw };
  }
  if (typeof raw === "object" && raw !== null && !Array.isArray(raw)) {
    return { kind: "inline", value: parseContract(raw) };
  }
  throw new ManifestParseError(
    "contract must be an inline object or a file path string"
  );
}

function parseAdapters(raw: unknown): Record<string, AdapterConfig> {
  if (raw === undefined || raw === null) return {};
  if (typeof raw !== "object" || Array.isArray(raw)) {
    throw new ManifestParseError("adapters must be an object");
  }
  const result: Record<string, AdapterConfig> = {};
  for (const [key, val] of Object.entries(raw as Record<string, unknown>)) {
    if (typeof val !== "object" || val === null || Array.isArray(val)) {
      throw new ManifestParseError(`adapters.${key} must be an object`);
    }
    const obj = val as Record<string, unknown>;
    if (typeof obj.enabled !== "boolean") {
      throw new ManifestParseError(`adapters.${key}.enabled is required and must be a boolean`);
    }
    result[key] = obj as AdapterConfig;
  }
  return result;
}

function parseDependencies(raw: unknown): Dependencies | undefined {
  if (raw === undefined || raw === null) return undefined;
  if (typeof raw !== "object" || Array.isArray(raw)) {
    throw new ManifestParseError("dependencies must be an object");
  }
  const obj = raw as Record<string, unknown>;
  const deps: Dependencies = {};

  if (obj.skills !== undefined) {
    if (!Array.isArray(obj.skills)) {
      throw new ManifestParseError("dependencies.skills must be an array");
    }
    deps.skills = obj.skills.map((s: unknown, i: number) => {
      if (typeof s !== "object" || s === null) {
        throw new ManifestParseError(`dependencies.skills[${i}] must be an object`);
      }
      const sObj = s as Record<string, unknown>;
      if (typeof sObj.name !== "string") {
        throw new ManifestParseError(`dependencies.skills[${i}].name is required`);
      }
      return {
        name: sObj.name,
        version: typeof sObj.version === "string" ? sObj.version : undefined,
      };
    });
  }

  if (obj.tools !== undefined) {
    if (!Array.isArray(obj.tools)) {
      throw new ManifestParseError("dependencies.tools must be an array");
    }
    deps.tools = obj.tools.map((t: unknown, i: number) => {
      if (typeof t !== "object" || t === null) {
        throw new ManifestParseError(`dependencies.tools[${i}] must be an object`);
      }
      const tObj = t as Record<string, unknown>;
      if (typeof tObj.name !== "string") {
        throw new ManifestParseError(`dependencies.tools[${i}].name is required`);
      }
      return {
        name: tObj.name,
        version: typeof tObj.version === "string" ? tObj.version : undefined,
      };
    });
  }

  return deps;
}

function parseMetadata(raw: unknown): ManifestMetadata | undefined {
  if (raw === undefined || raw === null) return undefined;
  if (typeof raw !== "object" || Array.isArray(raw)) {
    throw new ManifestParseError("metadata must be an object");
  }
  const obj = raw as Record<string, unknown>;
  const meta: ManifestMetadata = {};

  if (obj.author !== undefined) meta.author = String(obj.author);
  if (obj.license !== undefined) meta.license = String(obj.license);
  if (obj.homepage !== undefined) meta.homepage = String(obj.homepage);
  if (obj.repository !== undefined) meta.repository = String(obj.repository);

  if (obj.tags !== undefined) {
    if (!Array.isArray(obj.tags)) {
      throw new ManifestParseError("metadata.tags must be an array");
    }
    meta.tags = obj.tags.map((t: unknown) => String(t));
  }

  // Pass through extra fields
  for (const [key, val] of Object.entries(obj)) {
    if (!["author", "license", "homepage", "repository", "tags"].includes(key)) {
      meta[key] = val;
    }
  }

  return meta;
}

// --- Parsing ---

/**
 * Parse a skill manifest from a YAML string.
 * Matches the behavior of parse_manifest() in the Rust crate.
 */
export function parseManifest(yamlString: string): Manifest {
  let raw: unknown;
  try {
    raw = YAML.parse(yamlString);
  } catch (err) {
    throw new ManifestParseError(
      `YAML parse error: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new ManifestParseError("manifest must be a YAML mapping");
  }

  const obj = raw as Record<string, unknown>;

  // Required fields
  if (typeof obj.schemaVersion !== "string") {
    throw new ManifestParseError("schemaVersion is required and must be a string");
  }
  if (typeof obj.name !== "string") {
    throw new ManifestParseError("name is required and must be a string");
  }
  if (typeof obj.description !== "string") {
    throw new ManifestParseError("description is required and must be a string");
  }
  if (typeof obj.version !== "string") {
    throw new ManifestParseError("version is required and must be a string");
  }

  // content (required)
  if (typeof obj.content !== "object" || obj.content === null || Array.isArray(obj.content)) {
    throw new ManifestParseError("content is required and must be an object");
  }
  const contentObj = obj.content as Record<string, unknown>;
  if (typeof contentObj.skill !== "string") {
    throw new ManifestParseError("content.skill is required and must be a string");
  }
  const content: ContentPaths = { skill: contentObj.skill };
  if (contentObj.commands !== undefined) {
    if (typeof contentObj.commands !== "object" || contentObj.commands === null || Array.isArray(contentObj.commands)) {
      throw new ManifestParseError("content.commands must be an object");
    }
    content.commands = contentObj.commands as Record<string, string>;
  }

  // contract (required)
  if (obj.contract === undefined || obj.contract === null) {
    throw new ManifestParseError("contract is required");
  }

  return {
    schemaVersion: obj.schemaVersion,
    name: obj.name,
    description: obj.description,
    version: obj.version,
    content,
    contract: parseContractRef(obj.contract),
    identity: typeof obj.identity === "string" ? obj.identity : undefined,
    adapters: parseAdapters(obj.adapters),
    dependencies: parseDependencies(obj.dependencies),
    metadata: parseMetadata(obj.metadata),
    extensions:
      typeof obj.extensions === "object" && obj.extensions !== null && !Array.isArray(obj.extensions)
        ? (obj.extensions as Record<string, unknown>)
        : undefined,
  };
}

// --- Validation ---

const KNOWN_ADAPTER_TARGETS = new Set(["claude-code", "codex"]);

/**
 * Validate a parsed manifest, returning errors and warnings.
 * Matches validate_manifest() in the Rust crate.
 */
export function validateManifest(manifest: Manifest): ManifestValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // schemaVersion check
  if (manifest.schemaVersion !== "0.1.0") {
    errors.push(`schemaVersion must be "0.1.0", got "${manifest.schemaVersion}"`);
  }

  // name: kebab-case, 1-100 chars
  if (manifest.name.length === 0 || manifest.name.length > 100) {
    errors.push("name must be 1-100 characters");
  } else if (!isKebabCase(manifest.name)) {
    errors.push(
      `name must be kebab-case (lowercase alphanumeric and hyphens), got "${manifest.name}"`
    );
  }

  // description: 1-500 chars
  if (manifest.description.length === 0 || manifest.description.length > 500) {
    errors.push("description must be 1-500 characters");
  }

  // version: semver
  if (!isSemver(manifest.version)) {
    errors.push(`version must be valid semver, got "${manifest.version}"`);
  }

  // identity format (optional)
  if (manifest.identity !== undefined) {
    if (!isValidIdentity(manifest.identity)) {
      errors.push(
        `identity must be a valid domain/path string, got "${manifest.identity}"`
      );
    }
  }

  // tags limit
  if (manifest.metadata?.tags) {
    if (manifest.metadata.tags.length > 10) {
      errors.push(
        `tags must have at most 10 entries, got ${manifest.metadata.tags.length}`
      );
    }
  }

  // unknown adapter targets (warning, not error)
  for (const target of Object.keys(manifest.adapters)) {
    if (!KNOWN_ADAPTER_TARGETS.has(target)) {
      warnings.push(`unknown adapter target "${target}", will be skipped`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

// --- v0.2.0 Parsing ---

/**
 * Parse a v0.2.0 manifest from raw YAML object.
 */
function parseManifestV2(obj: Record<string, unknown>): ManifestV2 {
  if (typeof obj.name !== "string") throw new ManifestParseError("name is required");
  if (typeof obj.description !== "string") throw new ManifestParseError("description is required");
  if (typeof obj.version !== "string") throw new ManifestParseError("version is required");
  if (!Array.isArray(obj.files)) throw new ManifestParseError("files is required and must be an array");
  if (typeof obj.skills !== "object" || obj.skills === null || Array.isArray(obj.skills)) {
    throw new ManifestParseError("skills is required and must be an object");
  }

  const skills: Record<string, SkillDefinition> = {};
  for (const [name, raw] of Object.entries(obj.skills as Record<string, unknown>)) {
    if (typeof raw !== "object" || raw === null) {
      throw new ManifestParseError(`skills.${name} must be an object`);
    }
    const s = raw as Record<string, unknown>;
    if (typeof s.description !== "string") throw new ManifestParseError(`skills.${name}.description is required`);
    if (typeof s.entrypoint !== "string") throw new ManifestParseError(`skills.${name}.entrypoint is required`);
    if (typeof s.version !== "string") throw new ManifestParseError(`skills.${name}.version is required`);

    skills[name] = {
      description: s.description,
      entrypoint: s.entrypoint,
      version: s.version,
      inputs: s.inputs,
      outputs: s.outputs,
      permissions: Array.isArray(s.permissions) ? s.permissions.map(String) : undefined,
      determinism: typeof s.determinism === "string" ? s.determinism as SkillDefinition["determinism"] : undefined,
      errors: Array.isArray(s.errors) ? s.errors as SkillDefinition["errors"] : undefined,
      behavior: typeof s.behavior === "object" ? s.behavior as SkillDefinition["behavior"] : undefined,
      commands: typeof s.commands === "object" && s.commands !== null
        ? s.commands as Record<string, string>
        : undefined,
    };
  }

  let tools: Record<string, Tool> | undefined;
  if (obj.tools !== undefined && obj.tools !== null) {
    if (typeof obj.tools !== "object" || Array.isArray(obj.tools)) {
      throw new ManifestParseError("tools must be an object");
    }
    tools = {};
    for (const [name, raw] of Object.entries(obj.tools as Record<string, unknown>)) {
      if (typeof raw !== "object" || raw === null) {
        throw new ManifestParseError(`tools.${name} must be an object`);
      }
      const t = raw as Record<string, unknown>;
      if (typeof t.description !== "string") throw new ManifestParseError(`tools.${name}.description is required`);
      if (typeof t.using !== "string") throw new ManifestParseError(`tools.${name}.using is required`);
      if (typeof t.entrypoint !== "string") throw new ManifestParseError(`tools.${name}.entrypoint is required`);

      tools[name] = {
        description: t.description,
        using: t.using,
        version: typeof t.version === "string" ? t.version : undefined,
        entrypoint: t.entrypoint,
        input: typeof t.input === "object" && t.input !== null ? t.input as Record<string, unknown> : undefined,
        output: typeof t.output === "object" && t.output !== null ? t.output as Record<string, unknown> : undefined,
      };
    }
  }

  let hooks: Hooks | undefined;
  if (obj.hooks !== undefined && obj.hooks !== null) {
    if (typeof obj.hooks !== "object" || Array.isArray(obj.hooks)) {
      throw new ManifestParseError("hooks must be an object");
    }
    const h = obj.hooks as Record<string, unknown>;
    hooks = {
      onInstall: typeof h.onInstall === "string" ? h.onInstall : undefined,
      onActivate: typeof h.onActivate === "string" ? h.onActivate : undefined,
      onUninstall: typeof h.onUninstall === "string" ? h.onUninstall : undefined,
    };
  }

  return {
    schemaVersion: String(obj.schemaVersion),
    name: obj.name,
    description: obj.description,
    version: obj.version,
    files: obj.files.map(String),
    skills,
    tools,
    hooks,
    identity: typeof obj.identity === "string" ? obj.identity : undefined,
    adapters: parseAdapters(obj.adapters),
    dependencies: parseDependencies(obj.dependencies),
    metadata: parseMetadata(obj.metadata),
    extensions:
      typeof obj.extensions === "object" && obj.extensions !== null && !Array.isArray(obj.extensions)
        ? (obj.extensions as Record<string, unknown>)
        : undefined,
  };
}

/**
 * Parse a manifest of any schema version (v0.1.0 or v0.2.0).
 */
export function parseManifestAny(yamlString: string): ManifestAny {
  let raw: unknown;
  try {
    raw = YAML.parse(yamlString);
  } catch (err) {
    throw new ManifestParseError(
      `YAML parse error: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new ManifestParseError("manifest must be a YAML mapping");
  }

  const obj = raw as Record<string, unknown>;
  const schemaVersion = typeof obj.schemaVersion === "string" ? obj.schemaVersion : "0.1.0";

  switch (schemaVersion) {
    case "0.1.0":
      return { version: "0.1.0", manifest: parseManifest(yamlString) };
    case "0.2.0":
      if ("content" in obj) {
        throw new ManifestParseError(
          "v0.2.0 manifests must not contain 'content' — use 'files' and skill entrypoints instead"
        );
      }
      if ("contract" in obj) {
        throw new ManifestParseError(
          "v0.2.0 manifests must not contain 'contract' — use 'skills' instead"
        );
      }
      return { version: "0.2.0", manifest: parseManifestV2(obj) };
    default:
      throw new ManifestParseError(
        `unsupported schemaVersion "${schemaVersion}": supported versions are "0.1.0" and "0.2.0"`
      );
  }
}

// --- v0.2.0 Validation ---

const KNOWN_RUNTIMES = new Set(["node", "python", "shell"]);

/**
 * Validate a v0.2.0 manifest.
 */
export function validateManifestV2(manifest: ManifestV2): ManifestValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (manifest.schemaVersion !== "0.2.0") {
    errors.push(`schemaVersion must be "0.2.0", got "${manifest.schemaVersion}"`);
  }

  if (manifest.name.length === 0 || manifest.name.length > 100) {
    errors.push("name must be 1-100 characters");
  } else if (!isKebabCase(manifest.name)) {
    errors.push(`name must be kebab-case, got "${manifest.name}"`);
  }

  if (manifest.description.length === 0 || manifest.description.length > 500) {
    errors.push("description must be 1-500 characters");
  }

  if (!isSemver(manifest.version)) {
    errors.push(`version must be valid semver, got "${manifest.version}"`);
  }

  if (manifest.files.length === 0) {
    warnings.push("files list is empty — the package bundles no files");
  }

  if (Object.keys(manifest.skills).length === 0) {
    errors.push("skills map must contain at least one skill");
  }

  for (const [name, skill] of Object.entries(manifest.skills)) {
    if (!isKebabCase(name)) {
      errors.push(`skill name must be kebab-case, got "${name}"`);
    }
    if (!isSemver(skill.version)) {
      errors.push(`skills.${name}.version must be valid semver, got "${skill.version}"`);
    }
  }

  if (manifest.tools) {
    for (const [name, tool] of Object.entries(manifest.tools)) {
      if (!isKebabCase(name)) {
        errors.push(`tool name must be kebab-case, got "${name}"`);
      }
      if (!KNOWN_RUNTIMES.has(tool.using)) {
        warnings.push(`tools.${name}: unknown runtime "${tool.using}"`);
      }
    }
  }

  if (manifest.identity !== undefined && !isValidIdentity(manifest.identity)) {
    errors.push(`identity must be a valid domain/path string, got "${manifest.identity}"`);
  }

  if (manifest.metadata?.tags && manifest.metadata.tags.length > 10) {
    errors.push(`tags must have at most 10 entries, got ${manifest.metadata.tags.length}`);
  }

  for (const target of Object.keys(manifest.adapters)) {
    if (!KNOWN_ADAPTER_TARGETS.has(target)) {
      warnings.push(`unknown adapter target "${target}", will be skipped`);
    }
  }

  return { valid: errors.length === 0, errors, warnings };
}

/**
 * Validate any manifest version.
 */
export function validateManifestAny(manifestAny: ManifestAny): ManifestValidationResult {
  if (manifestAny.version === "0.1.0") {
    return validateManifest(manifestAny.manifest);
  }
  return validateManifestV2(manifestAny.manifest);
}
