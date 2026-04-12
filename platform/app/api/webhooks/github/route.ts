import { NextRequest, NextResponse } from "next/server";
import { createHmac, timingSafeEqual } from "crypto";
import { createAdminClient } from "@/lib/supabase/admin";
import { indexSkill } from "@/lib/indexer";

function verifySignature(payload: string, signature: string, secret: string): boolean {
  const expected = `sha256=${createHmac("sha256", secret).update(payload).digest("hex")}`;
  try {
    return timingSafeEqual(Buffer.from(signature), Buffer.from(expected));
  } catch {
    return false;
  }
}

export async function POST(request: NextRequest) {
  const secret = process.env.GITHUB_WEBHOOK_SECRET;
  if (!secret) {
    return NextResponse.json(
      { error: { code: "INTERNAL_ERROR", message: "Webhook secret not configured" } },
      { status: 500 }
    );
  }

  // Verify signature
  const signature = request.headers.get("x-hub-signature-256");
  if (!signature) {
    return NextResponse.json(
      { error: { code: "UNAUTHORIZED", message: "Missing signature" } },
      { status: 401 }
    );
  }

  const body = await request.text();
  if (!verifySignature(body, signature, secret)) {
    return NextResponse.json(
      { error: { code: "UNAUTHORIZED", message: "Invalid signature" } },
      { status: 401 }
    );
  }

  // Only handle push events
  const event = request.headers.get("x-github-event");
  if (event !== "push") {
    return NextResponse.json({ ok: true, message: `Ignored event: ${event}` });
  }

  const payload = JSON.parse(body);
  const repoFullName = payload.repository?.full_name;
  const ref = payload.ref; // e.g., "refs/heads/main"

  if (!repoFullName || !ref) {
    return NextResponse.json({ ok: true, message: "Missing repo or ref" });
  }

  // Extract branch name from ref
  const branch = ref.replace("refs/heads/", "");
  const repoUrl = `https://github.com/${repoFullName}`;

  const supabase = createAdminClient();

  // Find skills matching this repo and ref
  const { data: skills } = await supabase
    .from("skills")
    .select("id, repo_owner, repo_name, skill_path, ref, publisher_id")
    .eq("repo_url", repoUrl)
    .eq("ref", branch);

  if (!skills || skills.length === 0) {
    return NextResponse.json({ ok: true, message: "No matching skills" });
  }

  // Re-index each matching skill
  const results = [];
  for (const skill of skills) {
    const result = await indexSkill({
      skillId: skill.id,
      repoOwner: skill.repo_owner,
      repoName: skill.repo_name,
      skillPath: skill.skill_path,
      ref: skill.ref,
      publisherId: skill.publisher_id,
    });
    results.push({ skill_id: skill.id, ...result });
  }

  return NextResponse.json({ ok: true, indexed: results });
}
