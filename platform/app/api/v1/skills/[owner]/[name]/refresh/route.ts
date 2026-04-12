import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { requireAuth, notFound, forbidden } from "@/lib/api";
import { indexSkill } from "@/lib/indexer";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ owner: string; name: string }> }
) {
  const authResult = await requireAuth(request);
  if (authResult instanceof NextResponse) return authResult;
  const publisher = authResult;

  const { owner, name } = await params;
  const registryName = `@${owner}/${name}`;
  const supabase = createAdminClient();

  const { data: skill } = await supabase
    .from("skills")
    .select("id, publisher_id, repo_owner, repo_name, skill_path, ref")
    .eq("registry_name", registryName)
    .single();

  if (!skill) {
    return notFound(`Skill ${registryName} not found`);
  }

  if (skill.publisher_id !== publisher.id) {
    return forbidden("You can only refresh your own skills");
  }

  const result = await indexSkill({
    skillId: skill.id,
    repoOwner: skill.repo_owner,
    repoName: skill.repo_name,
    skillPath: skill.skill_path,
    ref: skill.ref,
    publisherId: publisher.id,
  });

  return NextResponse.json(result);
}
