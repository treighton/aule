// Permission vocabulary matching crates/aule-schema/src/permissions.rs

export type RiskTier = "none" | "low" | "medium" | "high";

const RISK_TIER_ORDER: Record<RiskTier, number> = {
  none: 0,
  low: 1,
  medium: 2,
  high: 3,
};

export interface PermissionDef {
  permission: string;
  category: string;
  scope: string;
  riskTier: RiskTier;
}

export const V0_VOCABULARY: readonly PermissionDef[] = [
  {
    permission: "filesystem.read",
    category: "filesystem",
    scope: "read",
    riskTier: "low",
  },
  {
    permission: "filesystem.write",
    category: "filesystem",
    scope: "write",
    riskTier: "high",
  },
  {
    permission: "filesystem.write.workspace",
    category: "filesystem",
    scope: "write.workspace",
    riskTier: "medium",
  },
  {
    permission: "network.external",
    category: "network",
    scope: "external",
    riskTier: "medium",
  },
  {
    permission: "network.external.specific",
    category: "network",
    scope: "external.specific",
    riskTier: "medium",
  },
  {
    permission: "process.spawn",
    category: "process",
    scope: "spawn",
    riskTier: "high",
  },
  {
    permission: "process.spawn.specific",
    category: "process",
    scope: "spawn.specific",
    riskTier: "medium",
  },
  {
    permission: "runtime.context",
    category: "runtime",
    scope: "context",
    riskTier: "low",
  },
] as const;

const VOCABULARY_MAP = new Map<string, PermissionDef>(
  V0_VOCABULARY.map((def) => [def.permission, def])
);

export interface PermissionCheck {
  validFormat: boolean;
  known: boolean;
  riskTier: RiskTier | null;
}

/**
 * Validate a permission string against the v0 vocabulary.
 * Format: lowercase alphanumeric + dots, no leading/trailing/consecutive dots.
 */
export function validatePermission(perm: string): PermissionCheck {
  const validFormat =
    perm.length > 0 &&
    /^[a-z0-9.]+$/.test(perm) &&
    !perm.startsWith(".") &&
    !perm.endsWith(".") &&
    !perm.includes("..");

  if (!validFormat) {
    return { validFormat: false, known: false, riskTier: null };
  }

  const def = VOCABULARY_MAP.get(perm);
  if (def) {
    return { validFormat: true, known: true, riskTier: def.riskTier };
  }

  return { validFormat: true, known: false, riskTier: null };
}

/**
 * Compute the maximum risk tier from a list of permissions.
 * Unknown permissions are ignored (risk = none).
 */
export function maxRiskTier(permissions: string[]): RiskTier {
  let max: RiskTier = "none";
  for (const perm of permissions) {
    const check = validatePermission(perm);
    if (check.riskTier && RISK_TIER_ORDER[check.riskTier] > RISK_TIER_ORDER[max]) {
      max = check.riskTier;
    }
  }
  return max;
}

/**
 * Check if `granted` permission implies `required` permission.
 * A broader scope implies all narrower scopes in the same category.
 * e.g., "filesystem.write" implies "filesystem.write.workspace"
 */
export function impliesPermission(granted: string, required: string): boolean {
  if (granted === required) return true;
  return required.startsWith(granted) && required[granted.length] === ".";
}
