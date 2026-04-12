import { NextRequest, NextResponse } from "next/server";
import { startDeviceAuth } from "@/lib/auth";

export async function POST(request: NextRequest) {
  const baseUrl = request.nextUrl.origin;
  const result = await startDeviceAuth(baseUrl);
  return NextResponse.json(result);
}
