import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { requireAuth, validationError, forbidden, internalError } from "@/lib/api";
import { verifyNamespaceOwnership } from "@/lib/auth";
import { indexSkill } from "@/lib/indexer";

export async function POST(request: NextRequest) {
  const authResult = await requireAuth(request);
  if (authResult instanceof NextResponse) return authResult;
  const publisher = authResult;

  let body: { repo_url?: string; skill_path?: string; ref?: string };
  try {
    body = await request.json();
  } catch {
    return validationError("Invalid JSON body");
  }

  const { repo_url, skill_path = ".", ref = "main" } = body;
  if (!repo_url) {
    return validationError("Missing required field: repo_url");
  }

  // Parse repo URL to extract owner and name
  const match = repo_url.match(
    /github\.com\/([^/]+)\/([^/]+?)(?:\.git)?$/
  );
  if (!match) {
    return validationError("Invalid GitHub repository URL", [
      { field: "repo_url", message: "Must be a GitHub repository URL (https://github.com/owner/repo)" },
    ]);
  }
  const [, repoOwner, repoName] = match;

  // Verify namespace ownership
  const githubToken = process.env.GITHUB_TOKEN;
  if (!githubToken) {
    return internalError("GitHub token not configured");
  }

  const ownsNamespace = await verifyNamespaceOwnership(
    publisher.github_username,
    repoOwner,
    githubToken
  );
  if (!ownsNamespace) {
    return forbidden(
      `You don't have permission to publish skills under the "${repoOwner}" namespace`
    );
  }

  const supabase = createAdminClient();

  // Fetch skill.yaml to get the skill name before creating the record
  const { fetchFileContent } = await import("@/lib/github");
  const manifestPathStr = skill_path === "." ? "skill.yaml" : `${skill_path}/skill.yaml`;
  let manifestYaml: string | null;
  try {
    manifestYaml = await fetchFileContent(repoOwner, repoName, manifestPathStr, ref);
  } catch {
    return validationError("Could not access repository", [
      { field: "repo_url", message: "Repository not accessible or does not exist" },
    ]);
  }

  if (!manifestYaml) {
    return validationError("No skill.yaml found", [
      { field: "skill_path", message: `No skill.yaml found at ${manifestPathStr}` },
    ]);
  }

  // Parse manifest to get name
  const { parseManifest } = await import("@/lib/manifest");
  let manifest;
  try {
    manifest = parseManifest(manifestYaml);
  } catch (err) {
    return validationError("Invalid skill.yaml", [
      { field: "skill_path", message: err instanceof Error ? err.message : String(err) },
    ]);
  }

  const skillName = manifest.name;
  const registryName = `@${repoOwner}/${skillName}`;

  // Check if skill already exists
  const { data: existing } = await supabase
    .from("skills")
    .select("id")
    .eq("registry_name", registryName)
    .single();

  if (existing) {
    return validationError("Skill already registered", [
      { field: "registry_name", message: `${registryName} is already registered` },
    ]);
  }

  // Create skill record
  const { data: skill, error: insertError } = await supabase
    .from("skills")
    .insert({
      publisher_id: publisher.id,
      name: skillName,
      registry_name: registryName,
      repo_url,
      repo_owner: repoOwner,
      repo_name: repoName,
      skill_path,
      ref,
      discovery_source: "submitted",
    })
    .select("id")
    .single();

  if (insertError || !skill) {
    return internalError("Failed to create skill record");
  }

  // Run indexing
  const indexResult = await indexSkill({
    skillId: skill.id,
    repoOwner,
    repoName,
    skillPath: skill_path,
    ref,
    publisherId: publisher.id,
  });

  // Fetch the full skill record to return
  const { data: fullSkill } = await supabase
    .from("skills")
    .select(
      `
      registry_name,
      name,
      description,
      tags,
      license,
      repo_url,
      skill_path,
      ref,
      last_indexed_at,
      created_at,
      publisher:publishers(github_username, display_name, avatar_url),
      latest_version:skill_versions(version, adapter_targets, permissions, commit_sha, created_at),
      verification:verification_results(check_name, status, message)
    `
    )
    .eq("id", skill.id)
    .eq("skill_versions.is_latest", true)
    .single();

  return NextResponse.json(fullSkill, { status: 201 });
}
