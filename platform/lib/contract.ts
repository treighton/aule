// Contract types and parser matching crates/aule-schema/src/contract.rs

import { validatePermission } from "./permissions";

// --- Types ---

export type Determinism = "deterministic" | "bounded" | "probabilistic";

export type LatencyClass = "fast" | "moderate" | "slow";

export type CostClass = "free" | "low" | "medium" | "high";

/**
 * Input/Output can be the literal string "prompt" for prompt-based skills,
 * or a JSON Schema object for structured skills.
 */
export type InputOutput = "prompt" | Record<string, unknown>;

export interface ContractError {
  code: string;
  description: string;
}

export interface BehavioralMetadata {
  latencyClass?: LatencyClass;
  costClass?: CostClass;
  sideEffects?: boolean;
}

export interface Contract {
  version: string;
  inputs: InputOutput;
  outputs: InputOutput;
  permissions: string[];
  determinism: Determinism;
  errors?: ContractError[];
  behavior?: BehavioralMetadata;
}

// --- Errors ---

export class ContractParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ContractParseError";
  }
}

export class ContractValidationError extends Error {
  constructor(
    message: string,
    public readonly errors: string[]
  ) {
    super(message);
    this.name = "ContractValidationError";
  }
}

// --- Validation result ---

export interface ContractValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// --- Helpers ---

const VALID_DETERMINISM = new Set<string>(["deterministic", "bounded", "probabilistic"]);
const VALID_LATENCY = new Set<string>(["fast", "moderate", "slow"]);
const VALID_COST = new Set<string>(["free", "low", "medium", "high"]);

function isPrompt(value: unknown): value is "prompt" {
  return value === "prompt";
}

function isInputOutput(value: unknown): value is InputOutput {
  return isPrompt(value) || (typeof value === "object" && value !== null && !Array.isArray(value));
}

function isSemver(s: string): boolean {
  const parts = s.split(".");
  if (parts.length !== 3) return false;
  return parts.every((p) => /^\d+$/.test(p));
}

// --- Parsing ---

/**
 * Parse a contract from an unknown data value (e.g. inline from manifest YAML
 * or parsed from a standalone contract file).
 */
export function parseContract(data: unknown): Contract {
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    throw new ContractParseError("contract must be an object");
  }

  const obj = data as Record<string, unknown>;

  // version (required)
  if (typeof obj.version !== "string") {
    throw new ContractParseError("contract.version is required and must be a string");
  }

  // inputs (required)
  if (!("inputs" in obj)) {
    throw new ContractParseError("contract.inputs is required");
  }
  if (!isInputOutput(obj.inputs)) {
    throw new ContractParseError(
      'contract.inputs must be "prompt" or a JSON Schema object'
    );
  }

  // outputs (required)
  if (!("outputs" in obj)) {
    throw new ContractParseError("contract.outputs is required");
  }
  if (!isInputOutput(obj.outputs)) {
    throw new ContractParseError(
      'contract.outputs must be "prompt" or a JSON Schema object'
    );
  }

  // permissions (default [])
  const rawPermissions = obj.permissions;
  let permissions: string[] = [];
  if (rawPermissions !== undefined) {
    if (!Array.isArray(rawPermissions)) {
      throw new ContractParseError("contract.permissions must be an array");
    }
    for (const p of rawPermissions) {
      if (typeof p !== "string") {
        throw new ContractParseError("each permission must be a string");
      }
    }
    permissions = rawPermissions as string[];
  }

  // determinism (default "probabilistic")
  let determinism: Determinism = "probabilistic";
  if (obj.determinism !== undefined) {
    if (typeof obj.determinism !== "string" || !VALID_DETERMINISM.has(obj.determinism)) {
      throw new ContractParseError(
        `contract.determinism must be one of: deterministic, bounded, probabilistic — got "${obj.determinism}"`
      );
    }
    determinism = obj.determinism as Determinism;
  }

  // errors (optional)
  let errors: ContractError[] | undefined;
  if (obj.errors !== undefined) {
    if (!Array.isArray(obj.errors)) {
      throw new ContractParseError("contract.errors must be an array");
    }
    errors = [];
    for (const e of obj.errors) {
      if (typeof e !== "object" || e === null) {
        throw new ContractParseError("each contract error must be an object");
      }
      const errObj = e as Record<string, unknown>;
      if (typeof errObj.code !== "string" || typeof errObj.description !== "string") {
        throw new ContractParseError(
          "each contract error must have 'code' and 'description' strings"
        );
      }
      errors.push({ code: errObj.code, description: errObj.description });
    }
  }

  // behavior (optional)
  let behavior: BehavioralMetadata | undefined;
  if (obj.behavior !== undefined) {
    if (typeof obj.behavior !== "object" || obj.behavior === null) {
      throw new ContractParseError("contract.behavior must be an object");
    }
    const bObj = obj.behavior as Record<string, unknown>;
    behavior = {};

    if (bObj.latencyClass !== undefined) {
      if (typeof bObj.latencyClass !== "string" || !VALID_LATENCY.has(bObj.latencyClass)) {
        throw new ContractParseError(
          `behavior.latencyClass must be one of: fast, moderate, slow — got "${bObj.latencyClass}"`
        );
      }
      behavior.latencyClass = bObj.latencyClass as LatencyClass;
    }

    if (bObj.costClass !== undefined) {
      if (typeof bObj.costClass !== "string" || !VALID_COST.has(bObj.costClass)) {
        throw new ContractParseError(
          `behavior.costClass must be one of: free, low, medium, high — got "${bObj.costClass}"`
        );
      }
      behavior.costClass = bObj.costClass as CostClass;
    }

    if (bObj.sideEffects !== undefined) {
      if (typeof bObj.sideEffects !== "boolean") {
        throw new ContractParseError("behavior.sideEffects must be a boolean");
      }
      behavior.sideEffects = bObj.sideEffects;
    }
  }

  return {
    version: obj.version,
    inputs: obj.inputs as InputOutput,
    outputs: obj.outputs as InputOutput,
    permissions,
    determinism,
    errors,
    behavior,
  };
}

/**
 * Validate a parsed contract, returning errors and warnings.
 * Matches the validation logic in the Rust crate.
 */
export function validateContract(contract: Contract): ContractValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // version must be semver
  if (!isSemver(contract.version)) {
    errors.push(`contract version must be valid semver, got "${contract.version}"`);
  }

  // validate permissions against vocabulary
  for (const perm of contract.permissions) {
    const check = validatePermission(perm);
    if (!check.validFormat) {
      errors.push(`permission "${perm}" has invalid format`);
    } else if (!check.known) {
      warnings.push(`permission "${perm}" is not in the v0 vocabulary`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}
