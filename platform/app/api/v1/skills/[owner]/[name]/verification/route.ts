import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { notFound } from "@/lib/api";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ owner: string; name: string }> }
) {
  const { owner, name } = await params;
  const registryName = `@${owner}/${name}`;
  const supabase = createAdminClient();

  const { data: skill } = await supabase
    .from("skills")
    .select("id")
    .eq("registry_name", registryName)
    .single();

  if (!skill) {
    return notFound(`Skill ${registryName} not found`);
  }

  // Get the latest version
  const { data: latestVersion } = await supabase
    .from("skill_versions")
    .select("id, version")
    .eq("skill_id", skill.id)
    .eq("is_latest", true)
    .single();

  if (!latestVersion) {
    return notFound(`No versions found for ${registryName}`);
  }

  const { data: checks } = await supabase
    .from("verification_results")
    .select("check_name, status, message, created_at")
    .eq("skill_version_id", latestVersion.id);

  const results = checks ?? [];
  const hasError = results.some((c) => c.status === "error");
  const hasWarning = results.some((c) => c.status === "warning");

  return NextResponse.json({
    version: latestVersion.version,
    status: hasError ? "error" : hasWarning ? "warning" : "pass",
    checks: results,
  });
}
