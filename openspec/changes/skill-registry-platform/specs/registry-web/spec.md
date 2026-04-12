## Overview

Next.js App Router web application for the skill registry. Provides a human-friendly interface for discovering, browsing, and inspecting skills. Deployed on Vercel.

## Requirements

### Pages

**Landing / Search (`/`)**
- Hero section with tagline explaining the skill registry
- Prominent search bar
- Featured or recently published skills below the search bar
- Search bar submits to `/skills?q={query}` or performs client-side navigation

**Skill Browse / Search Results (`/skills`)**
- Search bar at top (pre-filled if `?q=` is present)
- Filter sidebar or bar:
  - Runtime target (Claude Code, Codex, etc.)
  - Tags (from indexed skills)
  - Verification status (all / verified only)
- Skill cards in a list or grid:
  - Skill name (`@owner/name`)
  - Description (truncated)
  - Tags
  - Runtime targets as badges
  - Verification status indicator
  - Publisher avatar + name
  - "Last updated" timestamp
- Pagination (offset-based, matching API)
- URL state: query params for `q`, `runtime`, `tags`, `page` — bookmarkable/shareable

**Skill Detail (`/skills/[owner]/[name]`)**
- Header: skill name, description, publisher info, verification badge
- Sidebar or top bar:
  - Install command: `skill install @owner/name` (click to copy)
  - Repository link
  - License
  - Version
  - Last indexed date
  - Runtime targets
  - Permissions list with risk indicators
- Main content area:
  - README / skill description (rendered markdown from skill.md or manifest description)
  - Contract summary (inputs, outputs, permissions, determinism)
  - Adapter targets with details
  - Version history (collapsible list from `/versions` endpoint)
  - Verification checks (list of checks with pass/warning/error status)
- SEO: server-rendered with appropriate meta tags and Open Graph data

**Publisher Profile (`/publishers/[name]`)**
- Publisher avatar, display name, bio, website link
- List of skills published by this publisher (same card format as search results)
- "Member since" date

**Auth Pages**
- `/auth/login` — "Sign in with GitHub" button, redirects to GitHub OAuth via Supabase
- `/auth/callback` — OAuth callback handler, redirects to dashboard or previous page
- `/auth/device` — Device authorization page for CLI flow: shows user_code, "Authorize" button, triggers OAuth

**Publisher Dashboard (authenticated)**
- `/dashboard` — list of publisher's own skills with management actions
- For each skill: refresh button, delete button, verification status, last indexed date
- "Register new skill" form: repo URL, skill path, ref inputs
- Not a complex dashboard — minimal viable management UI

### Design System

- Use a component library (e.g., shadcn/ui with Tailwind CSS) for consistent, accessible UI
- Dark mode support (follows system preference)
- Responsive: works on desktop and mobile
- Accessible: proper ARIA labels, keyboard navigation, color contrast

### Data Fetching

- Search results page: server-side fetch with streaming (React Server Components)
- Skill detail page: server-side fetch (good for SEO)
- Publisher page: server-side fetch
- Dashboard: client-side fetch with SWR or React Query (authenticated, no SEO needed)

### SEO

- Skill detail pages have unique `<title>` and `<meta description>` from skill metadata
- Open Graph tags for social sharing (skill name, description, publisher)
- Sitemap generation from indexed skills (can be a simple API route)
- robots.txt allowing search engine indexing of public pages

## Acceptance Criteria

- Landing page loads with search bar and featured skills
- Search returns relevant results with working filters and pagination
- Skill detail page shows complete metadata including install command, contract, verification
- Publisher profile shows their published skills
- GitHub OAuth login flow works end-to-end
- Device auth page accepts user_code and completes CLI authorization
- Dashboard allows registering, refreshing, and deleting skills
- Pages are server-rendered for SEO (skill detail, search results)
- Responsive layout works on mobile screens
- All interactive elements are keyboard accessible
