#!/usr/bin/env node

/**
 * Aggregate test results into a summary report.
 *
 * Input (JSON, first positional argument):
 *   { results: TestResult[] }
 *
 * Output (JSON, stdout):
 *   { status: "ok"|"error", summary: string, passRate: number }
 */

interface TestResult {
  name: string;
  method: string;
  path: string;
  passed: boolean;
  expectedStatus: number;
  actualStatus?: number;
  error?: string;
}

interface Input {
  results: TestResult[];
}

function main() {
  const rawInput = process.argv[2];
  if (!rawInput) {
    const error = {
      status: "error",
      message: "Missing input argument. Expected JSON: {results}",
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

  if (!Array.isArray(input.results)) {
    const error = { status: "error", message: "results must be an array" };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  const total = input.results.length;
  const passed = input.results.filter((r) => r.passed).length;
  const failed = total - passed;
  const passRate = total > 0 ? Math.round((passed / total) * 100) / 100 : 0;

  const lines: string[] = [];
  lines.push(`# API Contract Test Report`);
  lines.push(``);
  lines.push(`**Total:** ${total} tests`);
  lines.push(`**Passed:** ${passed}`);
  lines.push(`**Failed:** ${failed}`);
  lines.push(`**Pass rate:** ${(passRate * 100).toFixed(0)}%`);
  lines.push(``);

  if (failed > 0) {
    lines.push(`## Failures`);
    lines.push(``);
    for (const r of input.results.filter((r) => !r.passed)) {
      lines.push(`- **${r.method} ${r.path}** (${r.name})`);
      if (r.actualStatus !== undefined) {
        lines.push(
          `  Expected status ${r.expectedStatus}, got ${r.actualStatus}`
        );
      }
      if (r.error) {
        lines.push(`  Error: ${r.error}`);
      }
    }
  }

  const output = {
    status: "ok",
    summary: lines.join("\n"),
    passRate,
  };
  process.stdout.write(JSON.stringify(output) + "\n");
}

main();
