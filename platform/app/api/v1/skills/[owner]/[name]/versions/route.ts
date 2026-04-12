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

  const { data: versions } = await supabase
    .from("skill_versions")
    .select("version, is_latest, commit_sha, adapter_targets, created_at")
    .eq("skill_id", skill.id)
    .order("created_at", { ascending: false });

  return NextResponse.json({ versions: versions ?? [] });
}
