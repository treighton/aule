#!/usr/bin/env node

/**
 * Execute generated contract tests against a live API.
 *
 * Input (JSON, first positional argument):
 *   { testDir?: string, baseUrl: string }
 *
 * Output (JSON, stdout):
 *   { status: "ok"|"error", passed: number, failed: number, results: TestResult[] }
 */

import * as fs from "fs";
import * as path from "path";

interface Input {
  testDir?: string;
  baseUrl: string;
}

interface TestResult {
  name: string;
  method: string;
  path: string;
  passed: boolean;
  expectedStatus: number;
  actualStatus?: number;
  error?: string;
}

async function main() {
  const rawInput = process.argv[2];
  if (!rawInput) {
    const error = {
      status: "error",
      message: "Missing input argument. Expected JSON: {testDir?, baseUrl}",
    };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  let input: Input;
  try {
    input = JSON.parse(rawInput);
  } catch {
    const error = {
      status: "error",
      message: `Invalid JSON input: ${rawInput}`,
    };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  if (!input.baseUrl) {
    const error = { status: "error", message: "baseUrl is required" };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  const testDir = input.testDir || ".tests";
  if (!fs.existsSync(testDir)) {
    const error = {
      status: "error",
      message: `Test directory not found: ${testDir}. Run generate first.`,
    };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  const testFiles = fs
    .readdirSync(testDir)
    .filter((f) => f.endsWith(".test.json"));

  const results: TestResult[] = [];
  let passed = 0;
  let failed = 0;

  for (const file of testFiles) {
    const testDef = JSON.parse(
      fs.readFileSync(path.join(testDir, file), "utf-8")
    );
    const url = `${input.baseUrl.replace(/\/$/, "")}${testDef.path}`;

    try {
      const response = await fetch(url, { method: testDef.method });
      const testPassed = response.status === testDef.expectedStatus;

      if (testPassed) passed++;
      else failed++;

      results.push({
        name: file.replace(".test.json", ""),
        method: testDef.method,
        path: testDef.path,
        passed: testPassed,
        expectedStatus: testDef.expectedStatus,
        actualStatus: response.status,
      });
    } catch (err: any) {
      failed++;
      results.push({
        name: file.replace(".test.json", ""),
        method: testDef.method,
        path: testDef.path,
        passed: false,
        expectedStatus: testDef.expectedStatus,
        error: err.message || String(err),
      });
    }
  }

  const output = { status: "ok", passed, failed, results };
  process.stdout.write(JSON.stringify(output) + "\n");
}

main();
