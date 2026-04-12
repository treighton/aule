import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { notFound, getIntParam } from "@/lib/api";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ username: string }> }
) {
  const { username } = await params;
  const searchParams = request.nextUrl.searchParams;
  const limit = getIntParam(searchParams, "limit", 20, 100);
  const offset = getIntParam(searchParams, "offset", 0);

  const supabase = createAdminClient();

  // Verify publisher exists
  const { data: publisher } = await supabase
    .from("publishers")
    .select("id")
    .eq("github_username", username)
    .single();

  if (!publisher) {
    return notFound(`Publisher ${username} not found`);
  }

  const { data: skills, count } = await supabase
    .from("skills")
    .select(
      `
      registry_name,
      name,
      description,
      tags,
      repo_url,
      last_indexed_at,
      created_at,
      latest_version:skill_versions(version, adapter_targets)
    `,
      { count: "exact" }
    )
    .eq("publisher_id", publisher.id)
    .eq("skill_versions.is_latest", true)
    .order("created_at", { ascending: false })
    .range(offset, offset + limit - 1);

  return NextResponse.json({
    skills: skills ?? [],
    total: count ?? 0,
    limit,
    offset,
  });
}
