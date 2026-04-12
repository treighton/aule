import { NextRequest, NextResponse } from "next/server";
import { completeDeviceAuth } from "@/lib/auth";

export async function POST(request: NextRequest) {
  const body = await request.json();
  const { user_code, publisher_id } = body;

  if (!user_code || !publisher_id) {
    return NextResponse.json(
      { error: { code: "VALIDATION_ERROR", message: "user_code and publisher_id are required" } },
      { status: 422 }
    );
  }

  const result = await completeDeviceAuth(user_code, publisher_id);

  if (!result.success) {
    return NextResponse.json(
      { error: { code: "VALIDATION_ERROR", message: result.error ?? "Failed to complete auth" } },
      { status: 400 }
    );
  }

  return NextResponse.json({ success: true });
}
