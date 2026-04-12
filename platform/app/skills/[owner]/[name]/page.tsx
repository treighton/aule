import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { createAdminClient } from "@/lib/supabase/admin";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CopyButton } from "@/components/copy-button";

export const dynamic = "force-dynamic";

interface PageProps {
  params: Promise<{ owner: string; name: string }>;
}

async function getSkill(owner: string, name: string) {
  const supabase = createAdminClient();
  const registryName = `@${owner}/${name}`;

  const { data: skill } = await supabase
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
      skill_path,
      ref,
      homepage_url,
      last_indexed_at,
      created_at,
      publisher:publishers(id, github_username, display_name, avatar_url, bio),
      latest_version:skill_versions(
        id, version, manifest_snapshot, contract_snapshot, permissions, adapter_targets, commit_sha, created_at
      ),
      verification:verification_results(check_name, status, message, created_at)
    `
    )
    .eq("registry_name", registryName)
    .eq("skill_versions.is_latest", true)
    .single();

  return skill;
}

async function getVersions(skillId: string) {
  const supabase = createAdminClient();
  const { data } = await supabase
    .from("skill_versions")
    .select("version, is_latest, commit_sha, adapter_targets, created_at")
    .eq("skill_id", skillId)
    .order("created_at", { ascending: false });
  return data ?? [];
}

export async function generateMetadata({
  params,
}: PageProps): Promise<Metadata> {
  const { owner, name } = await params;
  const skill = await getSkill(owner, name);
  if (!skill) {
    return { title: "Skill not found" };
  }
  return {
    title: `@${owner}/${name}`,
    description:
      (skill.description as string) ??
      `Skill @${owner}/${name} on the Aule registry.`,
  };
}

export default async function SkillDetailPage({ params }: PageProps) {
  const { owner, name } = await params;
  const skill = await getSkill(owner, name);

  if (!skill) {
    notFound();
  }

  const rawPub = skill.publisher as unknown as Record<string, unknown> | null;
  const rawVer = Array.isArray(skill.latest_version)
    ? (skill.latest_version[0] as unknown as Record<string, unknown> | undefined)
    : (skill.latest_version as unknown as Record<string, unknown> | null);
  const checks =
    (skill.verification as unknown as Array<Record<string, unknown>>) ?? [];
  const hasError = checks.some((c) => c.status === "error");
  const hasWarning = checks.some((c) => c.status === "warning");
  const verificationStatus = hasError
    ? "error"
    : hasWarning
      ? "warning"
      : "pass";

  const versions = await getVersions(skill.id as string);

  const pub = rawPub
    ? {
        id: rawPub.id as string,
        github_username: rawPub.github_username as string,
        display_name: rawPub.display_name as string | null,
        avatar_url: rawPub.avatar_url as string | null,
      }
    : null;

  const contract = rawVer?.contract_snapshot as Record<string, unknown> | null;
  const permissions = (rawVer?.permissions as string[]) ?? [];
  const adapterTargets = (rawVer?.adapter_targets as string[]) ?? [];
  const version = (rawVer?.version as string) ?? null;
  const installCmd = `skill install @${owner}/${name}`;

  return (
    <div className="mx-auto max-w-6xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center gap-3">
          <h1 className="font-mono text-2xl font-semibold">
            @{owner}/{name}
          </h1>
          <VerificationBadge status={verificationStatus} />
        </div>
        {skill.description && (
          <p className="mt-2 max-w-2xl text-muted-foreground">
            {skill.description as string}
          </p>
        )}
        {pub && (
          <div className="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
            {pub.avatar_url && (
              <img
                src={pub.avatar_url}
                alt={pub.github_username}
                className="size-5 rounded-full"
              />
            )}
            <Link
              href={`/publishers/${pub.github_username}`}
              className="hover:text-foreground transition-colors"
            >
              {pub.display_name || pub.github_username}
            </Link>
          </div>
        )}
      </div>

      <div className="grid gap-8 lg:grid-cols-[1fr_280px]">
        {/* Main content */}
        <div>
          <Tabs defaultValue="overview">
            <TabsList variant="line">
              <TabsTrigger value="overview">Overview</TabsTrigger>
              <TabsTrigger value="versions">Versions</TabsTrigger>
              <TabsTrigger value="verification">Verification</TabsTrigger>
            </TabsList>

            <TabsContent value="overview" className="pt-4">
              {skill.description && (
                <div className="mb-6">
                  <h3 className="mb-2 text-sm font-medium">Description</h3>
                  <p className="text-sm text-muted-foreground">
                    {skill.description as string}
                  </p>
                </div>
              )}

              {contract && (
                <div className="mb-6">
                  <h3 className="mb-2 text-sm font-medium">Contract</h3>
                  <div className="space-y-3 text-sm">
                    {Boolean(contract.inputs) && (
                      <div>
                        <span className="text-muted-foreground">Inputs: </span>
                        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                          {typeof contract.inputs === "string"
                            ? contract.inputs
                            : JSON.stringify(contract.inputs)}
                        </code>
                      </div>
                    )}
                    {Boolean(contract.outputs) && (
                      <div>
                        <span className="text-muted-foreground">
                          Outputs:{" "}
                        </span>
                        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                          {typeof contract.outputs === "string"
                            ? contract.outputs
                            : JSON.stringify(contract.outputs)}
                        </code>
                      </div>
                    )}
                    {Boolean(contract.determinism) && (
                      <div>
                        <span className="text-muted-foreground">
                          Determinism:{" "}
                        </span>
                        <Badge variant="secondary" className="text-[11px]">
                          {String(contract.determinism)}
                        </Badge>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {(skill.tags as string[] | null)?.length ? (
                <div>
                  <h3 className="mb-2 text-sm font-medium">Tags</h3>
                  <div className="flex flex-wrap gap-1.5">
                    {(skill.tags as string[]).map((tag: string) => (
                      <Badge
                        key={tag}
                        variant="secondary"
                        className="text-[11px]"
                      >
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
              ) : null}
            </TabsContent>

            <TabsContent value="versions" className="pt-4">
              {versions.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Version</TableHead>
                      <TableHead>Commit</TableHead>
                      <TableHead>Targets</TableHead>
                      <TableHead>Published</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {versions.map(
                      (v: Record<string, unknown>, i: number) => (
                        <TableRow key={i}>
                          <TableCell className="font-mono">
                            {v.version as string}
                            {Boolean(v.is_latest) && (
                              <Badge
                                variant="secondary"
                                className="ml-2 text-[10px]"
                              >
                                latest
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell className="font-mono text-xs text-muted-foreground">
                            {(v.commit_sha as string)?.slice(0, 7) ?? "--"}
                          </TableCell>
                          <TableCell>
                            <div className="flex gap-1">
                              {(
                                (v.adapter_targets as string[]) ?? []
                              ).map((t: string) => (
                                <Badge
                                  key={t}
                                  variant="outline"
                                  className="text-[10px]"
                                >
                                  {t}
                                </Badge>
                              ))}
                            </div>
                          </TableCell>
                          <TableCell className="text-xs text-muted-foreground">
                            {v.created_at
                              ? new Date(
                                  v.created_at as string
                                ).toLocaleDateString()
                              : "--"}
                          </TableCell>
                        </TableRow>
                      )
                    )}
                  </TableBody>
                </Table>
              ) : (
                <p className="text-sm text-muted-foreground">
                  No versions published yet.
                </p>
              )}
            </TabsContent>

            <TabsContent value="verification" className="pt-4">
              {checks.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Check</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Message</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {checks.map(
                      (c: Record<string, unknown>, i: number) => (
                        <TableRow key={i}>
                          <TableCell className="font-mono text-xs">
                            {c.check_name as string}
                          </TableCell>
                          <TableCell>
                            <StatusBadge
                              status={c.status as string}
                            />
                          </TableCell>
                          <TableCell className="text-xs text-muted-foreground">
                            {(c.message as string) || "--"}
                          </TableCell>
                        </TableRow>
                      )
                    )}
                  </TableBody>
                </Table>
              ) : (
                <p className="text-sm text-muted-foreground">
                  No verification checks recorded.
                </p>
              )}
            </TabsContent>
          </Tabs>
        </div>

        {/* Sidebar */}
        <aside className="space-y-6">
          {/* Install */}
          <div>
            <h3 className="mb-2 text-sm font-medium">Install</h3>
            <div className="flex items-center gap-1 rounded-lg bg-muted px-3 py-2 font-mono text-xs">
              <span className="flex-1 truncate">{installCmd}</span>
              <CopyButton text={installCmd} />
            </div>
          </div>

          <Separator />

          {/* Metadata */}
          <div className="space-y-3 text-sm">
            {version && (
              <MetaRow label="Version" value={version} />
            )}
            {skill.license && (
              <MetaRow label="License" value={skill.license as string} />
            )}
            {skill.last_indexed_at && (
              <MetaRow
                label="Last indexed"
                value={new Date(
                  skill.last_indexed_at as string
                ).toLocaleDateString()}
              />
            )}
            {skill.repo_url && (
              <div className="flex items-start justify-between gap-2">
                <span className="text-muted-foreground">Repository</span>
                <a
                  href={skill.repo_url as string}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="truncate text-right hover:underline"
                >
                  {(skill.repo_url as string).replace(
                    "https://github.com/",
                    ""
                  )}
                </a>
              </div>
            )}
          </div>

          {adapterTargets.length > 0 && (
            <>
              <Separator />
              <div>
                <h3 className="mb-2 text-sm font-medium">Runtime targets</h3>
                <div className="flex flex-wrap gap-1.5">
                  {adapterTargets.map((t) => (
                    <Badge
                      key={t}
                      variant="outline"
                      className="text-[11px]"
                    >
                      {t}
                    </Badge>
                  ))}
                </div>
              </div>
            </>
          )}

          {permissions.length > 0 && (
            <>
              <Separator />
              <div>
                <h3 className="mb-2 text-sm font-medium">Permissions</h3>
                <ul className="space-y-1 text-xs text-muted-foreground">
                  {permissions.map((p) => (
                    <li key={p} className="font-mono">
                      {p}
                    </li>
                  ))}
                </ul>
              </div>
            </>
          )}
        </aside>
      </div>
    </div>
  );
}

function VerificationBadge({ status }: { status: string }) {
  if (status === "pass") {
    return (
      <Badge variant="secondary" className="text-[11px] text-green-500">
        verified
      </Badge>
    );
  }
  if (status === "warning") {
    return (
      <Badge variant="secondary" className="text-[11px] text-yellow-500">
        warnings
      </Badge>
    );
  }
  return (
    <Badge variant="secondary" className="text-[11px] text-red-500">
      errors
    </Badge>
  );
}

function StatusBadge({ status }: { status: string }) {
  const color =
    status === "pass"
      ? "text-green-500"
      : status === "warning"
        ? "text-yellow-500"
        : "text-red-500";
  return (
    <Badge variant="secondary" className={`text-[11px] ${color}`}>
      {status}
    </Badge>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-2">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-right">{value}</span>
    </div>
  );
}
