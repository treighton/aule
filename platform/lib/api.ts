import { NextRequest, NextResponse } from "next/server";
import { authenticateToken, type AuthenticatedPublisher } from "./auth";

// --- Error Response Helpers ---

export type ErrorCode =
  | "NOT_FOUND"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "VALIDATION_ERROR"
  | "RATE_LIMITED"
  | "INTERNAL_ERROR";

interface ErrorDetail {
  field: string;
  message: string;
}

export function errorResponse(
  code: ErrorCode,
  message: string,
  status: number,
  details?: ErrorDetail[]
) {
  const body: Record<string, unknown> = {
    error: { code, message, ...(details ? { details } : {}) },
  };
  return NextResponse.json(body, { status });
}

export function notFound(message: string) {
  return errorResponse("NOT_FOUND", message, 404);
}

export function unauthorized(message = "Authentication required") {
  return errorResponse("UNAUTHORIZED", message, 401);
}

export function forbidden(message: string) {
  return errorResponse("FORBIDDEN", message, 403);
}

export function validationError(message: string, details?: ErrorDetail[]) {
  return errorResponse("VALIDATION_ERROR", message, 422, details);
}

export function internalError(message = "Internal server error") {
  return errorResponse("INTERNAL_ERROR", message, 500);
}

// --- Auth Middleware ---

export async function requireAuth(
  request: NextRequest
): Promise<AuthenticatedPublisher | NextResponse> {
  const authHeader = request.headers.get("authorization");
  if (!authHeader?.startsWith("Bearer ")) {
    return unauthorized();
  }

  const token = authHeader.slice(7);
  const publisher = await authenticateToken(token);

  if (!publisher) {
    return unauthorized("Invalid or expired token");
  }

  return publisher;
}

// --- Query Param Helpers ---

export function getIntParam(
  searchParams: URLSearchParams,
  name: string,
  defaultValue: number,
  max?: number
): number {
  const raw = searchParams.get(name);
  if (!raw) return defaultValue;
  const parsed = parseInt(raw, 10);
  if (isNaN(parsed) || parsed < 0) return defaultValue;
  if (max && parsed > max) return max;
  return parsed;
}
