import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { requireAuth, notFound, forbidden } from "@/lib/api";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ owner: string; name: string }> }
) {
  const { owner, name } = await params;
  const registryName = `@${owner}/${name}`;
  const supabase = createAdminClient();

  const { data: skill, error } = await supabase
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
      homepage_url,
      last_indexed_at,
      created_at,
      publisher:publishers(github_username, display_name, avatar_url, bio),
      latest_version:skill_versions(
        version,
        manifest_snapshot,
        permissions,
        adapter_targets,
        commit_sha,
        created_at
      ),
      verification:verification_results(check_name, status, message)
    `
    )
    .eq("registry_name", registryName)
    .eq("skill_versions.is_latest", true)
    .single();

  if (error || !skill) {
    return notFound(`Skill ${registryName} not found`);
  }

  // Shape the response
  const ver = Array.isArray(skill.latest_version)
    ? skill.latest_version[0]
    : skill.latest_version;

  const checks = (skill.verification as Array<Record<string, unknown>>) ?? [];
  const hasError = checks.some((c) => c.status === "error");
  const hasWarning = checks.some((c) => c.status === "warning");

  return NextResponse.json({
    registry_name: skill.registry_name,
    name: skill.name,
    description: skill.description,
    publisher: skill.publisher,
    latest_version: ver
      ? {
          version: (ver as Record<string, unknown>).version,
          manifest: (ver as Record<string, unknown>).manifest_snapshot,
          permissions: (ver as Record<string, unknown>).permissions,
          adapter_targets: (ver as Record<string, unknown>).adapter_targets,
          commit_sha: (ver as Record<string, unknown>).commit_sha,
          created_at: (ver as Record<string, unknown>).created_at,
        }
      : null,
    tags: skill.tags,
    license: skill.license,
    repo_url: skill.repo_url,
    skill_path: skill.skill_path,
    homepage_url: skill.homepage_url,
    verification: {
      status: hasError ? "error" : hasWarning ? "warning" : "pass",
      checks,
    },
    last_indexed_at: skill.last_indexed_at,
    created_at: skill.created_at,
  });
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ owner: string; name: string }> }
) {
  const authResult = await requireAuth(request);
  if (authResult instanceof NextResponse) return authResult;
  const publisher = authResult;

  const { owner, name } = await params;
  const registryName = `@${owner}/${name}`;
  const supabase = createAdminClient();

  // Verify ownership
  const { data: skill } = await supabase
    .from("skills")
    .select("id, publisher_id")
    .eq("registry_name", registryName)
    .single();

  if (!skill) {
    return notFound(`Skill ${registryName} not found`);
  }

  if (skill.publisher_id !== publisher.id) {
    return forbidden("You can only delete your own skills");
  }

  await supabase.from("skills").delete().eq("id", skill.id);

  return new NextResponse(null, { status: 204 });
}
