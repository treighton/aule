import Link from "next/link";

export function SiteFooter() {
  return (
    <footer className="border-t">
      <div className="mx-auto flex h-12 max-w-6xl items-center justify-between px-4 text-xs text-muted-foreground">
        <span>Aule Skill Registry</span>
        <nav className="flex gap-4">
          <Link href="/skills" className="hover:text-foreground transition-colors">
            Skills
          </Link>
          <a
            href="https://github.com/aule-org/aule"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-foreground transition-colors"
          >
            GitHub
          </a>
        </nav>
      </div>
    </footer>
  );
}
