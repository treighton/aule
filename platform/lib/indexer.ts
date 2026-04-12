import { createHash } from "crypto";
import { createAdminClient } from "./supabase/admin";
import { fetchFileContent, fetchRepoMetadata, fetchLatestCommitSha, fileExists } from "./github";
import { parseManifest, validateManifest, type Manifest, type ContractRef } from "./manifest";
import { type Contract, validateContract } from "./contract";
import { validatePermission } from "./permissions";

// --- Types ---

export interface IndexRequest {
  skillId: string;
  repoOwner: string;
  repoName: string;
  skillPath: string;
  ref: string;
  publisherId: string;
}

export type IndexResultStatus = "indexed" | "updated" | "unchanged" | "failed";

export interface IndexResult {
  status: IndexResultStatus;
  version?: string;
  verificationChecks: VerificationCheck[];
  error?: string;
}

export interface VerificationCheck {
  checkName: string;
  status: "pass" | "warning" | "error";
  message?: string;
}

// --- Helpers ---

function sha256(content: string): string {
  return createHash("sha256").update(content).digest("hex");
}

function getInlineContract(ref: ContractRef | undefined): Contract | null {
  if (!ref) return null;
  if (ref.kind === "inline") return ref.value;
  return null;
}

function manifestPath(skillPath: string): string {
  return skillPath === "." ? "skill.yaml" : `${skillPath}/skill.yaml`;
}

function contentPath(skillPath: string): string {
  return skillPath === "." ? "content/skill.md" : `${skillPath}/content/skill.md`;
}

// --- Verification ---

function runVerificationChecks(
  manifest: Manifest,
  contractValid: boolean,
  contractWarnings: string[],
  manifestWarnings: string[],
  contentExists: boolean,
  hasDescription: boolean,
): VerificationCheck[] {
  const checks: VerificationCheck[] = [];

  // Manifest schema check
  if (manifestWarnings.length > 0) {
    checks.push({
      checkName: "manifest_schema",
      status: "warning",
      message: manifestWarnings.join("; "),
    });
  } else {
    checks.push({ checkName: "manifest_schema", status: "pass" });
  }

  // Contract schema check
  if (!contractValid) {
    checks.push({
      checkName: "contract_schema",
      status: "error",
      message: "Contract validation failed",
    });
  } else if (contractWarnings.length > 0) {
    checks.push({
      checkName: "contract_schema",
      status: "warning",
      message: contractWarnings.join("; "),
    });
  } else {
    checks.push({ checkName: "contract_schema", status: "pass" });
  }

  // Permission vocabulary check
  const permChecks: string[] = [];
  const inlineContract = getInlineContract(manifest.contract);
  const permissions = inlineContract?.permissions ?? [];
  for (const perm of permissions) {
    const result = validatePermission(perm);
    if (!result.validFormat) {
      permChecks.push(`Invalid permission format: ${perm}`);
    } else if (!result.known) {
      permChecks.push(`Unknown permission: ${perm}`);
    }
  }
  if (permChecks.length > 0) {
    checks.push({
      checkName: "permission_vocabulary",
      status: "warning",
      message: permChecks.join("; "),
    });
  } else {
    checks.push({ checkName: "permission_vocabulary", status: "pass" });
  }

  // Content files check
  checks.push({
    checkName: "content_files",
    status: contentExists ? "pass" : "error",
    message: contentExists ? undefined : "content/skill.md not found in repository",
  });

  // Description check
  checks.push({
    checkName: "description_present",
    status: hasDescription ? "pass" : "warning",
    message: hasDescription ? undefined : "No description in manifest",
  });

  return checks;
}

// --- Main Indexer ---

export async function indexSkill(request: IndexRequest): Promise<IndexResult> {
  const { skillId, repoOwner, repoName, skillPath, ref, publisherId } = request;
  const supabase = createAdminClient();
  const checks: VerificationCheck[] = [];

  // 1. Fetch latest commit SHA
  let commitSha: string;
  try {
    commitSha = await fetchLatestCommitSha(repoOwner, repoName, ref);
  } catch (err) {
    return {
      status: "failed",
      verificationChecks: [],
      error: `Failed to fetch commit SHA: ${err instanceof Error ? err.message : String(err)}`,
    };
  }

  // 2. Check if anything changed since last index
  const { data: skill } = await supabase
    .from("skills")
    .select("last_indexed_sha")
    .eq("id", skillId)
    .single();

  if (skill?.last_indexed_sha === commitSha) {
    return { status: "unchanged", verificationChecks: [] };
  }

  // 3. Fetch skill.yaml from GitHub
  let manifestYaml: string | null;
  try {
    manifestYaml = await fetchFileContent(repoOwner, repoName, manifestPath(skillPath), ref);
  } catch (err) {
    return {
      status: "failed",
      verificationChecks: [],
      error: `Failed to fetch skill.yaml: ${err instanceof Error ? err.message : String(err)}`,
    };
  }

  if (!manifestYaml) {
    return {
      status: "failed",
      verificationChecks: [],
      error: `skill.yaml not found at ${manifestPath(skillPath)}`,
    };
  }

  // 4. Parse and validate manifest
  let manifest: Manifest;
  try {
    manifest = parseManifest(manifestYaml);
  } catch (err) {
    return {
      status: "failed",
      verificationChecks: [
        {
          checkName: "manifest_schema",
          status: "error",
          message: err instanceof Error ? err.message : String(err),
        },
      ],
      error: `Manifest parse error: ${err instanceof Error ? err.message : String(err)}`,
    };
  }

  const manifestValidation = validateManifest(manifest);
  const manifestWarnings = manifestValidation.warnings ?? [];

  // 5. Validate contract (if inline)
  let contractValid = true;
  let contractWarnings: string[] = [];
  let contractSnapshot: unknown = null;

  const contract = getInlineContract(manifest.contract);
  if (contract) {
    try {
      const contractValidation = validateContract(contract);
      contractValid = contractValidation.valid;
      contractWarnings = contractValidation.warnings ?? [];
      contractSnapshot = contract;
    } catch {
      contractValid = false;
    }
  }

  // 6. Check content file exists
  const skillContentExists = await fileExists(
    repoOwner,
    repoName,
    contentPath(skillPath),
    ref
  );

  // 7. Fetch content for description extraction (if no manifest description)
  let description = manifest.description ?? null;
  if (!description && skillContentExists) {
    try {
      const content = await fetchFileContent(
        repoOwner, repoName, contentPath(skillPath), ref
      );
      if (content) {
        // Extract first paragraph as description
        const lines = content.split("\n").filter((l) => l.trim() && !l.startsWith("#") && !l.startsWith("---"));
        description = lines[0]?.trim()?.slice(0, 500) ?? null;
      }
    } catch {
      // Non-critical, continue without description
    }
  }

  // 8. Fetch repo metadata
  let repoMeta;
  try {
    repoMeta = await fetchRepoMetadata(repoOwner, repoName);
  } catch {
    repoMeta = null;
  }

  // 9. Run verification checks
  const hasDescription = Boolean(description);
  const verificationChecks = runVerificationChecks(
    manifest,
    contractValid,
    contractWarnings,
    manifestWarnings,
    skillContentExists,
    hasDescription,
  );

  // 10. Compute hashes
  const manifestHash = sha256(manifestYaml);
  const contentHash = skillContentExists
    ? sha256((await fetchFileContent(repoOwner, repoName, contentPath(skillPath), ref)) ?? "")
    : null;

  // 11. Extract metadata for filtering
  const allPermissions = getInlineContract(manifest.contract)?.permissions ?? [];
  const adapterTargets = manifest.adapters
    ? Object.keys(manifest.adapters)
    : [];
  const tags = manifest.metadata?.tags ?? [];

  // 12. Check if this version already exists
  const version = manifest.version ?? "0.0.0";

  const { data: existingVersions } = await supabase
    .from("skill_versions")
    .select("id, manifest_hash")
    .eq("skill_id", skillId)
    .eq("version", version);

  const existingVersion = existingVersions?.[0];
  let resultStatus: IndexResultStatus;

  if (existingVersion && existingVersion.manifest_hash === manifestHash) {
    // Same version, same content — just update the skill record timestamps
    resultStatus = "unchanged";
  } else if (existingVersion) {
    // Same version but content changed — update the version record
    await supabase
      .from("skill_versions")
      .update({
        manifest_hash: manifestHash,
        manifest_snapshot: manifest as unknown as Record<string, unknown>,
        contract_snapshot: contractSnapshot as Record<string, unknown> | null,
        permissions: allPermissions,
        adapter_targets: adapterTargets,
        content_hash: contentHash,
        commit_sha: commitSha,
      })
      .eq("id", existingVersion.id);

    resultStatus = "updated";
  } else {
    // New version — clear is_latest on other versions, insert new
    await supabase
      .from("skill_versions")
      .update({ is_latest: false })
      .eq("skill_id", skillId);

    await supabase.from("skill_versions").insert({
      skill_id: skillId,
      version,
      manifest_hash: manifestHash,
      manifest_snapshot: manifest as unknown as Record<string, unknown>,
      contract_snapshot: contractSnapshot as Record<string, unknown> | null,
      permissions: allPermissions,
      adapter_targets: adapterTargets,
      content_hash: contentHash,
      commit_sha: commitSha,
      is_latest: true,
    });

    resultStatus = "indexed";
  }

  // 13. Update skill record
  await supabase
    .from("skills")
    .update({
      description,
      tags,
      license: repoMeta?.license ?? null,
      homepage_url: manifest.metadata?.homepage ?? null,
      last_indexed_at: new Date().toISOString(),
      last_indexed_sha: commitSha,
    })
    .eq("id", skillId);

  // 14. Update search vector (call the Postgres function)
  await supabase.rpc("update_skill_search_vector", { p_skill_id: skillId });

  // 15. Store verification results
  const { data: latestVersion } = await supabase
    .from("skill_versions")
    .select("id")
    .eq("skill_id", skillId)
    .eq("is_latest", true)
    .single();

  if (latestVersion) {
    for (const check of verificationChecks) {
      await supabase.from("verification_results").upsert(
        {
          skill_version_id: latestVersion.id,
          check_name: check.checkName,
          status: check.status,
          message: check.message ?? null,
        },
        { onConflict: "skill_version_id,check_name" }
      );
    }
  }

  return {
    status: resultStatus,
    version,
    verificationChecks,
  };
}
