// GitHub API client for the skill indexing pipeline

// --- Types ---

export interface RepoMetadata {
  stars: number;
  description: string | null;
  license: string | null;
  defaultBranch: string;
}

// --- Errors ---

export class GitHubApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly retryable: boolean
  ) {
    super(message);
    this.name = "GitHubApiError";
  }
}

export class GitHubNotFoundError extends GitHubApiError {
  constructor(message: string) {
    super(message, 404, false);
    this.name = "GitHubNotFoundError";
  }
}

export class GitHubRateLimitError extends GitHubApiError {
  public readonly retryAfterMs: number;

  constructor(message: string, status: number, retryAfterMs: number) {
    super(message, status, true);
    this.name = "GitHubRateLimitError";
    this.retryAfterMs = retryAfterMs;
  }
}

// --- Helpers ---

const GITHUB_API_BASE = "https://api.github.com";

function getHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github.v3+json",
    "User-Agent": "aule-platform/0.1",
  };

  const token = process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  return headers;
}

function parseRetryAfter(response: Response): number {
  // GitHub may send Retry-After header (seconds) or x-ratelimit-reset (epoch seconds)
  const retryAfter = response.headers.get("retry-after");
  if (retryAfter) {
    const seconds = parseInt(retryAfter, 10);
    if (!isNaN(seconds)) return seconds * 1000;
  }

  const resetEpoch = response.headers.get("x-ratelimit-reset");
  if (resetEpoch) {
    const resetMs = parseInt(resetEpoch, 10) * 1000;
    const nowMs = Date.now();
    if (!isNaN(resetMs) && resetMs > nowMs) {
      return resetMs - nowMs;
    }
  }

  // Default: 60 seconds
  return 60_000;
}

async function handleResponse(response: Response, context: string): Promise<Response> {
  if (response.ok) return response;

  if (response.status === 404) {
    throw new GitHubNotFoundError(`${context}: not found`);
  }

  if (response.status === 403 || response.status === 429) {
    const retryAfterMs = parseRetryAfter(response);
    throw new GitHubRateLimitError(
      `${context}: rate limited (${response.status})`,
      response.status,
      retryAfterMs
    );
  }

  throw new GitHubApiError(
    `${context}: HTTP ${response.status}`,
    response.status,
    response.status >= 500
  );
}

async function githubFetch(
  url: string,
  context: string,
  options?: RequestInit
): Promise<Response> {
  let response: Response;
  try {
    response = await fetch(url, {
      ...options,
      headers: { ...getHeaders(), ...options?.headers },
    });
  } catch (err) {
    throw new GitHubApiError(
      `${context}: network error — ${err instanceof Error ? err.message : String(err)}`,
      0,
      true
    );
  }

  return handleResponse(response, context);
}

// --- Public API ---

/**
 * Fetch and base64-decode file content via the GitHub Contents API.
 * Returns null if the file does not exist (404).
 */
export async function fetchFileContent(
  owner: string,
  repo: string,
  path: string,
  ref: string
): Promise<string | null> {
  const url = `${GITHUB_API_BASE}/repos/${owner}/${repo}/contents/${path}?ref=${encodeURIComponent(ref)}`;

  let response: Response;
  try {
    response = await githubFetch(url, `fetchFileContent(${owner}/${repo}/${path}@${ref})`);
  } catch (err) {
    if (err instanceof GitHubNotFoundError) return null;
    throw err;
  }

  const data = (await response.json()) as Record<string, unknown>;

  if (data.type !== "file" || typeof data.content !== "string") {
    return null;
  }

  // GitHub returns base64-encoded content with newlines
  const base64 = (data.content as string).replace(/\n/g, "");
  return atob(base64);
}

/**
 * Fetch repository metadata: stars, description, license, default branch.
 */
export async function fetchRepoMetadata(
  owner: string,
  repo: string
): Promise<RepoMetadata> {
  const url = `${GITHUB_API_BASE}/repos/${owner}/${repo}`;
  const response = await githubFetch(url, `fetchRepoMetadata(${owner}/${repo})`);
  const data = (await response.json()) as Record<string, unknown>;

  const licenseObj = data.license as Record<string, unknown> | null;

  return {
    stars: typeof data.stargazers_count === "number" ? data.stargazers_count : 0,
    description: typeof data.description === "string" ? data.description : null,
    license: licenseObj && typeof licenseObj.spdx_id === "string" ? licenseObj.spdx_id : null,
    defaultBranch: typeof data.default_branch === "string" ? data.default_branch : "main",
  };
}

/**
 * Fetch the latest commit SHA on a given ref (branch, tag, or commit).
 */
export async function fetchLatestCommitSha(
  owner: string,
  repo: string,
  ref: string
): Promise<string> {
  const url = `${GITHUB_API_BASE}/repos/${owner}/${repo}/commits/${encodeURIComponent(ref)}`;
  const response = await githubFetch(
    url,
    `fetchLatestCommitSha(${owner}/${repo}@${ref})`,
    { headers: { Accept: "application/vnd.github.sha" } }
  );
  return (await response.text()).trim();
}

/**
 * Check if a file exists at the given path and ref.
 * Uses a HEAD request to avoid downloading content.
 */
export async function fileExists(
  owner: string,
  repo: string,
  path: string,
  ref: string
): Promise<boolean> {
  const url = `${GITHUB_API_BASE}/repos/${owner}/${repo}/contents/${path}?ref=${encodeURIComponent(ref)}`;

  try {
    await githubFetch(url, `fileExists(${owner}/${repo}/${path}@${ref})`, {
      method: "HEAD",
    });
    return true;
  } catch (err) {
    if (err instanceof GitHubNotFoundError) return false;
    throw err;
  }
}
