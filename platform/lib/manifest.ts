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
