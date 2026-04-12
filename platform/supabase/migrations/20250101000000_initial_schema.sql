-- =============================================================================
-- Aule Skill Registry — Initial Schema
-- =============================================================================
-- Creates the core tables, indexes, functions, triggers, and RLS policies
-- for the skill registry platform.
-- =============================================================================


-- ---------------------------------------------------------------------------
-- 1. Utility functions
-- ---------------------------------------------------------------------------

-- Auto-update updated_at on row modification
create or replace function update_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;


-- ---------------------------------------------------------------------------
-- 2. Tables
-- ---------------------------------------------------------------------------

-- Publishers (1:1 with Supabase auth users)
create table publishers (
  id            uuid primary key,  -- matches auth.users.id
  github_username text unique not null,
  github_id     bigint unique not null,
  display_name  text,
  avatar_url    text,
  bio           text,
  website_url   text,
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

comment on table publishers is 'Skill publishers, linked 1:1 with Supabase auth users via id.';

-- Skills
create table skills (
  id               uuid primary key default gen_random_uuid(),
  publisher_id     uuid not null references publishers(id),
  name             text not null,  -- kebab-case
  registry_name    text unique not null,  -- @{github_username}/{name}
  repo_url         text not null,
  repo_owner       text not null,
  repo_name        text not null,
  skill_path       text not null default '.',
  ref              text not null default 'main',
  description      text,
  tags             text[],
  license          text,
  homepage_url     text,
  discovery_source text not null default 'submitted'
    check (discovery_source in ('submitted', 'crawled', 'imported')),
  last_indexed_at  timestamptz,
  last_indexed_sha text,
  search_vector    tsvector,
  created_at       timestamptz not null default now(),
  updated_at       timestamptz not null default now(),

  unique (publisher_id, name)
);

comment on table skills is 'Registered skills. Each skill points to a repo + path containing a skill.yaml manifest.';

-- Skill versions
create table skill_versions (
  id                uuid primary key default gen_random_uuid(),
  skill_id          uuid not null references skills(id) on delete cascade,
  version           text not null,
  manifest_hash     text not null,  -- SHA-256 of skill.yaml
  manifest_snapshot jsonb not null,
  contract_snapshot jsonb,
  permissions       text[],
  adapter_targets   text[],
  content_hash      text,
  commit_sha        text not null,
  is_latest         boolean not null default false,
  created_at        timestamptz not null default now(),

  unique (skill_id, version)
);

comment on table skill_versions is 'Immutable version records. Each row captures a snapshot of the manifest at a specific commit.';

-- Verification results
create table verification_results (
  id                uuid primary key default gen_random_uuid(),
  skill_version_id  uuid not null references skill_versions(id) on delete cascade,
  check_name        text not null,
  status            text not null
    check (status in ('pass', 'warning', 'error')),
  message           text,
  created_at        timestamptz not null default now(),

  unique (skill_version_id, check_name)
);

comment on table verification_results is 'Automated verification check results for each skill version.';

-- Device auth codes (for CLI device-flow authentication)
create table device_auth_codes (
  id            uuid primary key default gen_random_uuid(),
  device_code   text unique not null,
  user_code     text unique not null,
  api_token     text,
  publisher_id  uuid references publishers(id),
  status        text not null default 'pending'
    check (status in ('pending', 'completed', 'expired')),
  expires_at    timestamptz not null,
  created_at    timestamptz not null default now()
);

comment on table device_auth_codes is 'OAuth device-flow codes for CLI login. Short-lived, cleaned up after expiry.';

-- API tokens (hashed, for programmatic access)
create table api_tokens (
  id            uuid primary key default gen_random_uuid(),
  publisher_id  uuid not null references publishers(id) on delete cascade,
  token_hash    text unique not null,  -- SHA-256 of the raw token
  name          text not null,
  last_used_at  timestamptz,
  expires_at    timestamptz,
  created_at    timestamptz not null default now()
);

comment on table api_tokens is 'Hashed API tokens for programmatic registry access. Raw tokens are never stored.';


-- ---------------------------------------------------------------------------
-- 3. Indexes
-- ---------------------------------------------------------------------------

-- Full-text search on skills
create index idx_skills_search_vector on skills using gin (search_vector);

-- Tag filtering
create index idx_skills_tags on skills using gin (tags);

-- Foreign key lookups
create index idx_skills_publisher_id on skills (publisher_id);
create index idx_skill_versions_skill_id on skill_versions (skill_id);
create index idx_verification_results_skill_version_id on verification_results (skill_version_id);

-- Fast lookup for the latest version of each skill
create index idx_skill_versions_latest on skill_versions (skill_id, is_latest)
  where is_latest = true;

-- Pending device auth codes (polled during login flow)
create index idx_device_auth_codes_pending on device_auth_codes (status)
  where status = 'pending';


-- ---------------------------------------------------------------------------
-- 4. Search vector update function
-- ---------------------------------------------------------------------------

create or replace function update_skill_search_vector(p_skill_id uuid)
returns void as $$
begin
  update skills
  set search_vector =
    setweight(to_tsvector('english', coalesce(name, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(description, '')), 'B') ||
    setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'B') ||
    setweight(to_tsvector('english', coalesce(
      (select github_username from publishers where id = publisher_id), ''
    )), 'C')
  where id = p_skill_id;
end;
$$ language plpgsql security definer;

comment on function update_skill_search_vector(uuid) is
  'Recompute the weighted search_vector for a given skill. Call after inserting or updating a skill.';


-- ---------------------------------------------------------------------------
-- 5. Triggers
-- ---------------------------------------------------------------------------

-- Auto-update updated_at on publishers
create trigger trg_publishers_updated_at
  before update on publishers
  for each row
  execute function update_updated_at();

-- Auto-update updated_at on skills
create trigger trg_skills_updated_at
  before update on skills
  for each row
  execute function update_updated_at();


-- ---------------------------------------------------------------------------
-- 6. Row-Level Security
-- ---------------------------------------------------------------------------

-- Enable RLS on all tables
alter table publishers          enable row level security;
alter table skills              enable row level security;
alter table skill_versions      enable row level security;
alter table verification_results enable row level security;
alter table device_auth_codes   enable row level security;
alter table api_tokens          enable row level security;

-- publishers: public read; write requires auth.uid() = id
create policy "publishers_select"
  on publishers for select
  using (true);

create policy "publishers_insert"
  on publishers for insert
  with check (auth.uid() = id);

create policy "publishers_update"
  on publishers for update
  using (auth.uid() = id)
  with check (auth.uid() = id);

create policy "publishers_delete"
  on publishers for delete
  using (auth.uid() = id);

-- skills: public read; write requires auth.uid() = publisher_id
create policy "skills_select"
  on skills for select
  using (true);

create policy "skills_insert"
  on skills for insert
  with check (auth.uid() = publisher_id);

create policy "skills_update"
  on skills for update
  using (auth.uid() = publisher_id)
  with check (auth.uid() = publisher_id);

create policy "skills_delete"
  on skills for delete
  using (auth.uid() = publisher_id);

-- skill_versions: public read; write requires publisher owns the parent skill
create policy "skill_versions_select"
  on skill_versions for select
  using (true);

create policy "skill_versions_insert"
  on skill_versions for insert
  with check (
    exists (
      select 1 from skills
      where skills.id = skill_id
        and skills.publisher_id = auth.uid()
    )
  );

create policy "skill_versions_update"
  on skill_versions for update
  using (
    exists (
      select 1 from skills
      where skills.id = skill_id
        and skills.publisher_id = auth.uid()
    )
  );

create policy "skill_versions_delete"
  on skill_versions for delete
  using (
    exists (
      select 1 from skills
      where skills.id = skill_id
        and skills.publisher_id = auth.uid()
    )
  );

-- verification_results: public read; write is service-role only
-- (No insert/update/delete policies means only service-role can write)
create policy "verification_results_select"
  on verification_results for select
  using (true);

-- device_auth_codes: service-role only for both read and write
-- (No policies means only service-role bypasses RLS)

-- api_tokens: read requires auth.uid() = publisher_id; write is service-role only
create policy "api_tokens_select"
  on api_tokens for select
  using (auth.uid() = publisher_id);
