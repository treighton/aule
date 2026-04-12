import Link from "next/link";
import { SearchBar } from "@/components/search-bar";

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-sm">
      <div className="mx-auto flex h-12 max-w-6xl items-center gap-4 px-4">
        <Link
          href="/"
          className="shrink-0 font-heading text-base font-semibold tracking-tight"
        >
          Aule
        </Link>
        <nav className="hidden items-center gap-4 text-sm text-muted-foreground sm:flex">
          <Link href="/skills" className="hover:text-foreground transition-colors">
            Skills
          </Link>
        </nav>
        <div className="ml-auto w-full max-w-xs">
          <SearchBar placeholder="Search skills..." />
        </div>
      </div>
    </header>
  );
}
