"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";

export function SearchBar({
  defaultValue = "",
  placeholder = "Search skills...",
  size = "default",
}: {
  defaultValue?: string;
  placeholder?: string;
  size?: "default" | "lg";
}) {
  const router = useRouter();
  const [query, setQuery] = useState(defaultValue);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = query.trim();
    if (trimmed) {
      router.push(`/skills?q=${encodeURIComponent(trimmed)}`);
    } else {
      router.push("/skills");
    }
  }

  return (
    <form onSubmit={handleSubmit} className="relative w-full">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={placeholder}
        className={
          size === "lg"
            ? "h-11 pl-10 pr-4 text-base rounded-xl"
            : "h-8 pl-9 pr-3"
        }
      />
    </form>
  );
}
