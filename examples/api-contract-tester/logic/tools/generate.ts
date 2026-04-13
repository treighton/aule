#!/usr/bin/env node

/**
 * Generate test harness from an OpenAPI spec.
 *
 * Input (JSON, first positional argument):
 *   { specPath: string, outputDir?: string }
 *
 * Output (JSON, stdout):
 *   { status: "ok"|"error", testCount?: number, files?: string[], message?: string }
 */

import * as fs from "fs";
import * as path from "path";

interface Input {
  specPath: string;
  outputDir?: string;
}

function main() {
  const rawInput = process.argv[2];
  if (!rawInput) {
    const error = {
      status: "error",
      message: "Missing input argument. Expected JSON: {specPath, outputDir?}",
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

  if (!input.specPath) {
    const error = { status: "error", message: "specPath is required" };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  // Read the OpenAPI spec
  let specContent: string;
  try {
    specContent = fs.readFileSync(input.specPath, "utf-8");
  } catch {
    const error = {
      status: "error",
      message: `Spec file not found: ${input.specPath}`,
    };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  let spec: any;
  try {
    spec = JSON.parse(specContent);
  } catch {
    // Try YAML (simplified — in production you'd use a YAML parser)
    const error = {
      status: "error",
      message: "Only JSON OpenAPI specs are supported in this example",
    };
    process.stdout.write(JSON.stringify(error) + "\n");
    process.exit(0);
  }

  const outputDir = input.outputDir || ".tests";
  fs.mkdirSync(outputDir, { recursive: true });

  const paths = spec.paths || {};
  const files: string[] = [];
  let testCount = 0;

  for (const [pathStr, methods] of Object.entries(paths)) {
    for (const [method, operation] of Object.entries(methods as Record<string, any>)) {
      if (["get", "post", "put", "patch", "delete"].includes(method)) {
        testCount++;
        const safeName = `${method}_${pathStr.replace(/[^a-zA-Z0-9]/g, "_")}`;
        const testFile = path.join(outputDir, `${safeName}.test.json`);

        const testDef = {
          method: method.toUpperCase(),
          path: pathStr,
          operationId: operation.operationId || null,
          expectedStatus: method === "post" ? 201 : 200,
          responseSchema: operation.responses?.["200"]?.content?.["application/json"]?.schema
            || operation.responses?.["201"]?.content?.["application/json"]?.schema
            || null,
        };

        fs.writeFileSync(testFile, JSON.stringify(testDef, null, 2));
        files.push(testFile);
      }
    }
  }

  const output = {
    status: "ok",
    testCount,
    files,
  };
  process.stdout.write(JSON.stringify(output) + "\n");
}

main();
