import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { getIntParam } from "@/lib/api";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const q = searchParams.get("q");
  const runtime = searchParams.get("runtime");
  const tags = searchParams.getAll("tags");
  const publisher = searchParams.get("publisher");
  const limit = getIntParam(searchParams, "limit", 20, 100);
  const offset = getIntParam(searchParams, "offset", 0);

  const supabase = createAdminClient();

  let query = supabase
    .from("skills")
    .select(
      `
      id,
      registry_name,
      name,
      description,
      tags,
      license,
      repo_url,
      last_indexed_at,
      created_at,
      publisher:publishers!inner(github_username, display_name, avatar_url),
      latest_version:skill_versions(version, adapter_targets)
    `,
      { count: "exact" }
    )
    .eq("skill_versions.is_latest", true);

  // Full-text search
  if (q) {
    query = query.textSearch("search_vector", q, {
      type: "websearch",
      config: "english",
    });
  }

  // Filters
  if (runtime) {
    query = query.contains("skill_versions.adapter_targets", [runtime]);
  }
  if (tags.length > 0) {
    query = query.contains("tags", tags);
  }
  if (publisher) {
    query = query.eq("publishers.github_username", publisher);
  }

  // Pagination
  query = query.range(offset, offset + limit - 1);

  // Order by relevance if searching, otherwise by newest
  if (q) {
    query = query.order("created_at", { ascending: false });
  } else {
    query = query.order("created_at", { ascending: false });
  }

  const { data, count, error } = await query;

  if (error) {
    return NextResponse.json(
      { error: { code: "INTERNAL_ERROR", message: error.message } },
      { status: 500 }
    );
  }

  // Shape the response
  const results = (data ?? []).map((skill: Record<string, unknown>) => {
    const pub = skill.publisher as Record<string, unknown> | null;
    const ver = Array.isArray(skill.latest_version)
      ? (skill.latest_version[0] as Record<string, unknown>)
      : (skill.latest_version as Record<string, unknown> | null);

    return {
      registry_name: skill.registry_name,
      name: skill.name,
      description: skill.description,
      publisher: pub
        ? {
            github_username: pub.github_username,
            display_name: pub.display_name,
            avatar_url: pub.avatar_url,
          }
        : null,
      version: ver?.version ?? null,
      tags: skill.tags,
      adapter_targets: ver?.adapter_targets ?? [],
      repo_url: skill.repo_url,
      last_indexed_at: skill.last_indexed_at,
      created_at: skill.created_at,
    };
  });

  return NextResponse.json({
    results,
    total: count ?? 0,
    limit,
    offset,
  });
}
