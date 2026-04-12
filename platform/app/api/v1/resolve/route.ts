import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { notFound, validationError, internalError } from "@/lib/api";

export async function POST(request: NextRequest) {
  let body: { skill?: string; version_constraint?: string; runtime?: string };
  try {
    body = await request.json();
  } catch {
    return validationError("Invalid JSON body");
  }

  const { skill, runtime } = body;
  if (!skill) {
    return validationError("Missing required field: skill");
  }

  const supabase = createAdminClient();

  // Look up the skill by registry_name
  const registryName = skill.startsWith("@") ? skill : `@${skill}`;

  const { data: skillRecord, error } = await supabase
    .from("skills")
    .select(
      `
      id,
      registry_name,
      repo_url,
      repo_owner,
      repo_name,
      skill_path,
      ref,
      latest_version:skill_versions!inner(
        version,
        manifest_hash,
        adapter_targets,
        permissions,
        commit_sha
      ),
      verification:verification_results(status)
    `
    )
    .eq("registry_name", registryName)
    .eq("skill_versions.is_latest", true)
    .single();

  if (error || !skillRecord) {
    return notFound(`Skill ${registryName} not found`);
  }

  const version = Array.isArray(skillRecord.latest_version)
    ? (skillRecord.latest_version[0] as Record<string, unknown>)
    : (skillRecord.latest_version as Record<string, unknown> | null);

  if (!version) {
    return notFound(`No versions found for ${registryName}`);
  }

  // Check runtime compatibility if requested
  const adapterTargets = (version.adapter_targets as string[]) ?? [];
  if (runtime && !adapterTargets.includes(runtime)) {
    return validationError(
      `No compatible adapter for runtime "${runtime}". Available: ${adapterTargets.join(", ")}`
    );
  }

  // Compute verification status
  const checks = (skillRecord.verification as Array<Record<string, unknown>>) ?? [];
  const hasError = checks.some((c) => c.status === "error");
  const hasWarning = checks.some((c) => c.status === "warning");
  const verificationStatus = hasError ? "error" : hasWarning ? "warning" : "pass";

  return NextResponse.json({
    resolved: {
      registry_name: skillRecord.registry_name,
      version: version.version,
      repo_url: skillRecord.repo_url,
      ref: skillRecord.ref,
      skill_path: skillRecord.skill_path,
      commit_sha: version.commit_sha,
      manifest_hash: version.manifest_hash,
      adapter_targets: adapterTargets,
      permissions: (version.permissions as string[]) ?? [],
      verification_status: verificationStatus,
    },
  });
}
