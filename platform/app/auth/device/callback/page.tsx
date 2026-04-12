"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
} from "@/components/ui/card";
import { createClient } from "@/lib/supabase/client";

function CallbackContent() {
  const searchParams = useSearchParams();
  const userCode = searchParams.get("user_code") ?? "";
  const [status, setStatus] = useState<"completing" | "success" | "error">(
    "completing"
  );
  const [errorMsg, setErrorMsg] = useState("");

  useEffect(() => {
    async function complete() {
      if (!userCode) {
        setStatus("error");
        setErrorMsg("Missing device code.");
        return;
      }

      try {
        const supabase = createClient();
        const {
          data: { session },
        } = await supabase.auth.getSession();

        if (!session) {
          setStatus("error");
          setErrorMsg("Authentication failed. Please try again.");
          return;
        }

        const res = await fetch("/api/v1/auth/device/complete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            user_code: userCode,
            publisher_id: session.user.id,
          }),
        });

        if (!res.ok) {
          const body = await res.json();
          setStatus("error");
          setErrorMsg(
            body?.error?.message ?? "Failed to complete authorization."
          );
          return;
        }

        setStatus("success");
      } catch (err) {
        setStatus("error");
        setErrorMsg(
          err instanceof Error ? err.message : "An unexpected error occurred."
        );
      }
    }
    complete();
  }, [userCode]);

  return (
    <Card className="w-full max-w-sm">
      <CardHeader>
        <CardTitle>CLI Authorization</CardTitle>
      </CardHeader>
      <CardContent>
        {status === "completing" && (
          <p className="text-sm text-muted-foreground">
            Completing authorization...
          </p>
        )}
        {status === "success" && (
          <div className="rounded-lg bg-green-500/10 p-4 text-center text-sm text-green-500">
            Authorization complete. You can close this window and return to
            your terminal.
          </div>
        )}
        {status === "error" && (
          <div className="rounded-lg bg-red-500/10 p-4 text-center text-sm text-red-500">
            {errorMsg}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default function DeviceCallbackPage() {
  return (
    <div className="flex min-h-[60vh] items-center justify-center px-4">
      <Suspense
        fallback={
          <Card className="w-full max-w-sm">
            <CardHeader>
              <CardTitle>CLI Authorization</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">Loading...</p>
            </CardContent>
          </Card>
        }
      >
        <CallbackContent />
      </Suspense>
    </div>
  );
}
