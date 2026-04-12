import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "@/components/ui/card";

export interface SkillCardData {
  registry_name: string;
  name: string;
  description: string | null;
  tags: string[] | null;
  publisher: {
    github_username: string;
    display_name: string | null;
    avatar_url: string | null;
  } | null;
  version: string | null;
  adapter_targets: string[] | null;
  verification_summary?: {
    status: string;
    checks_passed: number;
    checks_warned: number;
    checks_failed: number;
  } | null;
}

function VerificationDot({ status }: { status: string }) {
  const color =
    status === "pass"
      ? "bg-green-500"
      : status === "warning"
        ? "bg-yellow-500"
        : "bg-red-500";
  return (
    <span
      className={`inline-block size-2 rounded-full ${color}`}
      title={`Verification: ${status}`}
    />
  );
}

export function SkillCard({ skill }: { skill: SkillCardData }) {
  const [, owner, name] = skill.registry_name.match(/^@([^/]+)\/(.+)$/) ?? [
    null,
    "",
    skill.name,
  ];

  return (
    <Link href={`/skills/${owner}/${name}`} className="block">
      <Card className="hover:ring-foreground/20 transition-all">
        <CardHeader>
          <div className="flex items-center gap-2">
            <CardTitle className="font-mono text-sm">
              {skill.registry_name}
            </CardTitle>
            {skill.verification_summary && (
              <VerificationDot status={skill.verification_summary.status} />
            )}
            {skill.version && (
              <span className="text-xs text-muted-foreground">
                v{skill.version}
              </span>
            )}
          </div>
          {skill.description && (
            <CardDescription className="line-clamp-2">
              {skill.description}
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-1.5">
            {skill.tags?.slice(0, 5).map((tag) => (
              <Badge key={tag} variant="secondary" className="text-[11px]">
                {tag}
              </Badge>
            ))}
            {skill.adapter_targets?.map((target) => (
              <Badge key={target} variant="outline" className="text-[11px]">
                {target}
              </Badge>
            ))}
          </div>
        </CardContent>
        {skill.publisher && (
          <CardFooter className="gap-2 text-xs text-muted-foreground">
            {skill.publisher.avatar_url && (
              <img
                src={skill.publisher.avatar_url}
                alt={skill.publisher.github_username}
                className="size-4 rounded-full"
              />
            )}
            <span>{skill.publisher.display_name || skill.publisher.github_username}</span>
          </CardFooter>
        )}
      </Card>
    </Link>
  );
}
