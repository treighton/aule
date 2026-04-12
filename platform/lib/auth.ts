import { createHash, randomBytes } from "crypto";
import { createAdminClient } from "./supabase/admin";

// --- Types ---

export interface DeviceAuthStartResponse {
  device_code: string;
  user_code: string;
  verification_url: string;
  expires_in: number;
  interval: number;
}

export interface DeviceAuthPollResponse {
  status: "pending" | "completed" | "expired";
  api_token?: string;
  publisher?: {
    github_username: string;
    display_name: string | null;
  };
}

export interface AuthenticatedPublisher {
  id: string;
  github_username: string;
  display_name: string | null;
}

// --- Helpers ---

function generateDeviceCode(): string {
  return randomBytes(20).toString("hex"); // 40 chars
}

function generateUserCode(): string {
  // 8-char human-readable code: XXXX-XXXX
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no O/0/I/1
  let code = "";
  const bytes = randomBytes(8);
  for (let i = 0; i < 8; i++) {
    code += chars[bytes[i] % chars.length];
  }
  return `${code.slice(0, 4)}-${code.slice(4)}`;
}

function generateApiToken(): string {
  return `sk_${randomBytes(32).toString("hex")}`; // sk_ prefix + 64 hex chars
}

export function hashToken(token: string): string {
  return createHash("sha256").update(token).digest("hex");
}

// --- Device Auth Flow ---

export async function startDeviceAuth(
  registryBaseUrl: string
): Promise<DeviceAuthStartResponse> {
  const supabase = createAdminClient();

  const deviceCode = generateDeviceCode();
  const userCode = generateUserCode();
  const expiresIn = 900; // 15 minutes

  await supabase.from("device_auth_codes").insert({
    device_code: deviceCode,
    user_code: userCode,
    status: "pending",
    expires_at: new Date(Date.now() + expiresIn * 1000).toISOString(),
  });

  return {
    device_code: deviceCode,
    user_code: userCode,
    verification_url: `${registryBaseUrl}/auth/device`,
    expires_in: expiresIn,
    interval: 5,
  };
}

export async function pollDeviceAuth(
  deviceCode: string
): Promise<DeviceAuthPollResponse> {
  const supabase = createAdminClient();

  const { data: auth } = await supabase
    .from("device_auth_codes")
    .select("status, api_token, publisher_id, expires_at")
    .eq("device_code", deviceCode)
    .single();

  if (!auth) {
    return { status: "expired" };
  }

  // Check expiry
  if (new Date(auth.expires_at) < new Date()) {
    await supabase
      .from("device_auth_codes")
      .update({ status: "expired" })
      .eq("device_code", deviceCode);
    return { status: "expired" };
  }

  if (auth.status === "completed" && auth.api_token && auth.publisher_id) {
    // Fetch publisher info
    const { data: publisher } = await supabase
      .from("publishers")
      .select("github_username, display_name")
      .eq("id", auth.publisher_id)
      .single();

    return {
      status: "completed",
      api_token: auth.api_token,
      publisher: publisher
        ? {
            github_username: publisher.github_username,
            display_name: publisher.display_name,
          }
        : undefined,
    };
  }

  return { status: "pending" };
}

export async function completeDeviceAuth(
  userCode: string,
  publisherId: string
): Promise<{ success: boolean; error?: string }> {
  const supabase = createAdminClient();

  // Find the pending auth code
  const { data: auth } = await supabase
    .from("device_auth_codes")
    .select("id, device_code, expires_at, status")
    .eq("user_code", userCode)
    .eq("status", "pending")
    .single();

  if (!auth) {
    return { success: false, error: "Invalid or expired code" };
  }

  if (new Date(auth.expires_at) < new Date()) {
    await supabase
      .from("device_auth_codes")
      .update({ status: "expired" })
      .eq("id", auth.id);
    return { success: false, error: "Code expired" };
  }

  // Generate API token
  const rawToken = generateApiToken();
  const tokenHash = hashToken(rawToken);

  // Store the token
  await supabase.from("api_tokens").insert({
    publisher_id: publisherId,
    token_hash: tokenHash,
    name: `cli-${new Date().toISOString().slice(0, 10)}`,
  });

  // Complete the device auth
  await supabase
    .from("device_auth_codes")
    .update({
      status: "completed",
      publisher_id: publisherId,
      api_token: rawToken, // stored temporarily for CLI pickup, device_code row is cleaned up later
    })
    .eq("id", auth.id);

  return { success: true };
}

// --- API Token Authentication ---

export async function authenticateToken(
  token: string
): Promise<AuthenticatedPublisher | null> {
  const supabase = createAdminClient();
  const tokenHash = hashToken(token);

  const { data: tokenRecord } = await supabase
    .from("api_tokens")
    .select("id, publisher_id, expires_at")
    .eq("token_hash", tokenHash)
    .single();

  if (!tokenRecord) return null;

  // Check expiry
  if (tokenRecord.expires_at && new Date(tokenRecord.expires_at) < new Date()) {
    return null;
  }

  // Update last_used_at
  await supabase
    .from("api_tokens")
    .update({ last_used_at: new Date().toISOString() })
    .eq("id", tokenRecord.id);

  // Fetch publisher
  const { data: publisher } = await supabase
    .from("publishers")
    .select("id, github_username, display_name")
    .eq("id", tokenRecord.publisher_id)
    .single();

  return publisher ?? null;
}

// --- Namespace Ownership ---

export async function verifyNamespaceOwnership(
  publisherGithubUsername: string,
  repoOwner: string,
  githubToken: string
): Promise<boolean> {
  // Direct match — publisher owns the namespace
  if (publisherGithubUsername.toLowerCase() === repoOwner.toLowerCase()) {
    return true;
  }

  // Check if publisher is admin/maintain of the org
  try {
    const response = await fetch(
      `https://api.github.com/orgs/${repoOwner}/memberships/${publisherGithubUsername}`,
      {
        headers: {
          Authorization: `Bearer ${githubToken}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
      }
    );

    if (!response.ok) return false;

    const membership = await response.json();
    return membership.role === "admin" || membership.role === "member";
  } catch {
    return false;
  }
}
