import { createAdminClient } from "@/lib/supabase/admin";
import { SearchBar } from "@/components/search-bar";
import { SkillCard, type SkillCardData } from "@/components/skill-card";

export const dynamic = "force-dynamic";

export default async function Home() {
  const supabase = createAdminClient();

  const { data } = await supabase
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
    `
    )
    .eq("skill_versions.is_latest", true)
    .order("created_at", { ascending: false })
    .limit(6);

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

  return (
    <div className="flex flex-col items-center">
      {/* Hero */}
      <section className="flex w-full flex-col items-center gap-6 px-4 pt-24 pb-16 text-center">
        <h1 className="text-4xl font-semibold tracking-tight sm:text-5xl">
          Aule
        </h1>
        <p className="max-w-md text-lg text-muted-foreground">
          Discover, publish, and install skills for coding agents.
        </p>
        <div className="w-full max-w-lg">
          <SearchBar
            placeholder="Search skills..."
            size="lg"
          />
        </div>
      </section>

      {/* Recently published */}
      {skills.length > 0 && (
        <section className="w-full max-w-6xl px-4 pb-16">
          <h2 className="mb-4 text-sm font-medium text-muted-foreground">
            Recently published
          </h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {skills.map((skill) => (
              <SkillCard key={skill.registry_name} skill={skill} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
