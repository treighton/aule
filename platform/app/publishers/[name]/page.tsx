import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { createAdminClient } from "@/lib/supabase/admin";
import { SkillCard, type SkillCardData } from "@/components/skill-card";

export const dynamic = "force-dynamic";

interface PageProps {
  params: Promise<{ name: string }>;
}

async function getPublisher(username: string) {
  const supabase = createAdminClient();
  const { data } = await supabase
    .from("publishers")
    .select(
      "id, github_username, display_name, avatar_url, bio, website_url, created_at"
    )
    .eq("github_username", username)
    .single();
  return data;
}

async function getPublisherSkills(publisherId: string) {
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
    .eq("publisher_id", publisherId)
    .eq("skill_versions.is_latest", true)
    .order("created_at", { ascending: false });
  return data ?? [];
}

export async function generateMetadata({
  params,
}: PageProps): Promise<Metadata> {
  const { name } = await params;
  const publisher = await getPublisher(name);
  if (!publisher) {
    return { title: "Publisher not found" };
  }
  return {
    title: publisher.display_name || publisher.github_username,
    description: publisher.bio ?? `Skills published by ${publisher.github_username}`,
  };
}

export default async function PublisherPage({ params }: PageProps) {
  const { name } = await params;
  const publisher = await getPublisher(name);

  if (!publisher) {
    notFound();
  }

  const rawSkills = await getPublisherSkills(publisher.id);

  const skills: SkillCardData[] = rawSkills.map(
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
    <div className="mx-auto max-w-6xl px-4 py-8">
      {/* Profile header */}
      <div className="mb-8 flex items-start gap-4">
        {publisher.avatar_url && (
          <img
            src={publisher.avatar_url}
            alt={publisher.github_username}
            className="size-16 rounded-full"
          />
        )}
        <div>
          <h1 className="text-xl font-semibold">
            {publisher.display_name || publisher.github_username}
          </h1>
          <p className="text-sm text-muted-foreground">
            @{publisher.github_username}
          </p>
          {publisher.bio && (
            <p className="mt-1 max-w-lg text-sm text-muted-foreground">
              {publisher.bio}
            </p>
          )}
          <div className="mt-2 flex gap-4 text-xs text-muted-foreground">
            {publisher.website_url && (
              <a
                href={publisher.website_url}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-foreground transition-colors"
              >
                {publisher.website_url.replace(/^https?:\/\//, "")}
              </a>
            )}
            <span>
              Member since{" "}
              {new Date(publisher.created_at).toLocaleDateString()}
            </span>
          </div>
        </div>
      </div>

      {/* Skills grid */}
      <h2 className="mb-4 text-sm font-medium text-muted-foreground">
        Published skills ({skills.length})
      </h2>
      {skills.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {skills.map((skill) => (
            <SkillCard key={skill.registry_name} skill={skill} />
          ))}
        </div>
      ) : (
        <p className="py-12 text-center text-sm text-muted-foreground">
          No skills published yet.
        </p>
      )}
    </div>
  );
}
