import Link from "next/link";
import { createAdminClient } from "@/lib/supabase/admin";
import { SearchBar } from "@/components/search-bar";
import { SkillCard, type SkillCardData } from "@/components/skill-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export const dynamic = "force-dynamic";

const RUNTIMES = ["claude-code", "codex"];
const PAGE_SIZE = 12;

export default async function SkillsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  const q = typeof sp.q === "string" ? sp.q : "";
  const runtime = typeof sp.runtime === "string" ? sp.runtime : "";
  const pageStr = typeof sp.page === "string" ? sp.page : "1";
  const page = Math.max(1, parseInt(pageStr, 10) || 1);
  const offset = (page - 1) * PAGE_SIZE;

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
      created_at,
      publisher:publishers!inner(github_username, display_name, avatar_url),
      latest_version:skill_versions!inner(version, adapter_targets),
      verification:verification_results(check_name, status)
    `,
      { count: "exact" }
    )
    .eq("skill_versions.is_latest", true);

  if (q) {
    query = query.textSearch("search_vector", q, {
      type: "websearch",
      config: "english",
    });
  }

  if (runtime) {
    query = query.contains("skill_versions.adapter_targets", [runtime]);
  }

  query = query
    .order("created_at", { ascending: false })
    .range(offset, offset + PAGE_SIZE - 1);

  const { data, count } = await query;
  const total = count ?? 0;
  const totalPages = Math.ceil(total / PAGE_SIZE);

  const skills: SkillCardData[] = (data ?? []).map(
    (skill: Record<string, unknown>) => {
      const pub = skill.publisher as Record<string, unknown> | null;
      const ver = Array.isArray(skill.latest_version)
        ? (skill.latest_version[0] as Record<string, unknown>)
        : (skill.latest_version as Record<string, unknown> | null);
      const checks =
        (skill.verification as Array<Record<string, unknown>>) ?? [];
      const checksFailed = checks.filter((c) => c.status === "error").length;
      const checksWarned = checks.filter((c) => c.status === "warning").length;

      return {
        registry_name: skill.registry_name as string,
        name: skill.name as string,
        description: skill.description as string | null,
        tags: skill.tags as string[] | null,
        publisher: pub
          ? {
              github_username: pub.github_username as string,
              display_name: pub.display_name as string | null,
              avatar_url: pub.avatar_url as string | null,
            }
          : null,
        version: (ver?.version as string) ?? null,
        adapter_targets: (ver?.adapter_targets as string[]) ?? null,
        verification_summary: {
          status:
            checksFailed > 0
              ? "error"
              : checksWarned > 0
                ? "warning"
                : "pass",
          checks_passed: checks.filter((c) => c.status === "pass").length,
          checks_warned: checksWarned,
          checks_failed: checksFailed,
        },
      };
    }
  );

  function buildUrl(overrides: Record<string, string>) {
    const params = new URLSearchParams();
    if (q) params.set("q", q);
    if (runtime) params.set("runtime", runtime);
    for (const [k, v] of Object.entries(overrides)) {
      if (v) {
        params.set(k, v);
      } else {
        params.delete(k);
      }
    }
    const qs = params.toString();
    return `/skills${qs ? `?${qs}` : ""}`;
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-8">
      {/* Search */}
      <div className="mb-6 max-w-lg">
        <SearchBar defaultValue={q} placeholder="Search skills..." size="lg" />
      </div>

      {/* Runtime filters */}
      <div className="mb-6 flex flex-wrap gap-2">
        <Link href={buildUrl({ runtime: "" })}>
          <Badge variant={!runtime ? "default" : "secondary"}>All</Badge>
        </Link>
        {RUNTIMES.map((rt) => (
          <Link key={rt} href={buildUrl({ runtime: rt })}>
            <Badge variant={runtime === rt ? "default" : "secondary"}>
              {rt}
            </Badge>
          </Link>
        ))}
      </div>

      {/* Results count */}
      <p className="mb-4 text-sm text-muted-foreground">
        {total} skill{total !== 1 ? "s" : ""} found
        {q ? ` for "${q}"` : ""}
      </p>

      {/* Skill list */}
      {skills.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {skills.map((skill) => (
            <SkillCard key={skill.registry_name} skill={skill} />
          ))}
        </div>
      ) : (
        <p className="py-12 text-center text-muted-foreground">
          No skills found. Try a different search.
        </p>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-8 flex items-center justify-center gap-2">
          {page > 1 && (
            <Link href={buildUrl({ page: String(page - 1) })}>
              <Button variant="outline" size="sm">
                Previous
              </Button>
            </Link>
          )}
          <span className="text-sm text-muted-foreground">
            Page {page} of {totalPages}
          </span>
          {page < totalPages && (
            <Link href={buildUrl({ page: String(page + 1) })}>
              <Button variant="outline" size="sm">
                Next
              </Button>
            </Link>
          )}
        </div>
      )}
    </div>
  );
}
