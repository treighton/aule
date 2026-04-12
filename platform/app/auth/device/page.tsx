"use client";

import { Suspense, useState, useEffect } from "react";
import { useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { createClient } from "@/lib/supabase/client";

function DeviceAuthForm() {
  const searchParams = useSearchParams();
  const codeFromUrl = searchParams.get("code") ?? "";

  const [userCode, setUserCode] = useState(codeFromUrl);
  const [status, setStatus] = useState<
    "idle" | "authenticating" | "completing" | "success" | "error"
  >("idle");
  const [errorMsg, setErrorMsg] = useState("");

  useEffect(() => {
    if (codeFromUrl) {
      setUserCode(codeFromUrl);
    }
  }, [codeFromUrl]);

  async function handleAuthorize() {
    if (!userCode.trim()) {
      setErrorMsg("Please enter the code displayed in your CLI.");
      return;
    }

    setStatus("authenticating");
    setErrorMsg("");

    try {
      const supabase = createClient();
      const { data, error } = await supabase.auth.signInWithOAuth({
        provider: "github",
        options: {
          redirectTo: `${window.location.origin}/auth/device/callback?user_code=${encodeURIComponent(userCode.trim())}`,
        },
      });

      if (error) {
        setStatus("error");
        setErrorMsg(error.message);
        return;
      }

      if (data.url) {
        window.location.href = data.url;
      }
    } catch (err) {
      setStatus("error");
      setErrorMsg(
        err instanceof Error ? err.message : "An unexpected error occurred."
      );
    }
  }

  return (
    <Card className="w-full max-w-sm">
      <CardHeader>
        <CardTitle>Authorize CLI Access</CardTitle>
        <CardDescription>
          Enter the code displayed in your terminal to link your account.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {status === "success" ? (
          <div className="rounded-lg bg-green-500/10 p-4 text-center text-sm text-green-500">
            Authorization complete. You can close this window and return to
            your terminal.
          </div>
        ) : (
          <>
            <div className="space-y-2">
              <Label htmlFor="user_code">Device code</Label>
              <Input
                id="user_code"
                value={userCode}
                onChange={(e) => setUserCode(e.target.value)}
                placeholder="XXXX-XXXX"
                className="font-mono text-center text-lg tracking-widest"
                maxLength={9}
              />
            </div>

            {errorMsg && (
              <p className="text-sm text-red-500">{errorMsg}</p>
            )}

            <Button
              className="w-full"
              onClick={handleAuthorize}
              disabled={status === "authenticating" || status === "completing"}
            >
              {status === "authenticating" || status === "completing"
                ? "Authorizing..."
                : "Authorize"}
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}

export default function DeviceAuthPage() {
  return (
    <div className="flex min-h-[60vh] items-center justify-center px-4">
      <Suspense
        fallback={
          <Card className="w-full max-w-sm">
            <CardHeader>
              <CardTitle>Authorize CLI Access</CardTitle>
              <CardDescription>Loading...</CardDescription>
            </CardHeader>
          </Card>
        }
      >
        <DeviceAuthForm />
      </Suspense>
    </div>
  );
}
