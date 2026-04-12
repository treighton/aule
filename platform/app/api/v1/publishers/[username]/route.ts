import { NextRequest, NextResponse } from "next/server";
import { createAdminClient } from "@/lib/supabase/admin";
import { notFound } from "@/lib/api";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ username: string }> }
) {
  const { username } = await params;
  const supabase = createAdminClient();

  const { data: publisher } = await supabase
    .from("publishers")
    .select("github_username, display_name, avatar_url, bio, website_url, created_at")
    .eq("github_username", username)
    .single();

  if (!publisher) {
    return notFound(`Publisher ${username} not found`);
  }

  return NextResponse.json(publisher);
}
