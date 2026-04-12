import { NextRequest, NextResponse } from "next/server";
import { pollDeviceAuth } from "@/lib/auth";
import { validationError } from "@/lib/api";

export async function POST(request: NextRequest) {
  let body: { device_code?: string };
  try {
    body = await request.json();
  } catch {
    return validationError("Invalid JSON body");
  }

  if (!body.device_code) {
    return validationError("Missing required field: device_code");
  }

  const result = await pollDeviceAuth(body.device_code);
  return NextResponse.json(result);
}
