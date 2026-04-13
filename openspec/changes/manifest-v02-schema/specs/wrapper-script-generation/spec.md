## ADDED Requirements

### Requirement: Wrapper script generation per tool
At `skill build` time, the adapter SHALL generate one executable shell wrapper script per declared tool. The wrapper SHALL be placed in a `tools/` directory within the generated skill directory.

#### Scenario: Node.js tool wrapper
- **WHEN** a tool declares `using: "node"` and `entrypoint: "logic/tools/generate.ts"`
- **THEN** the adapter SHALL generate a wrapper script at `tools/generate` containing:
  ```
  #!/bin/sh
  exec node "$(dirname "$0")/../logic/tools/generate.ts" "$@"
  ```
- **THEN** the wrapper SHALL be marked executable (chmod +x)

#### Scenario: Python tool wrapper
- **WHEN** a tool declares `using: "python"` and `entrypoint: "logic/tools/analyze.py"`
- **THEN** the adapter SHALL generate a wrapper script at `tools/analyze` containing:
  ```
  #!/bin/sh
  exec python3 "$(dirname "$0")/../logic/tools/analyze.py" "$@"
  ```

#### Scenario: Shell tool wrapper
- **WHEN** a tool declares `using: "shell"` and `entrypoint: "logic/tools/cleanup.sh"`
- **THEN** the adapter SHALL generate a wrapper script at `tools/cleanup` containing:
  ```
  #!/bin/sh
  exec "$(dirname "$0")/../logic/tools/cleanup.sh" "$@"
  ```

### Requirement: Tool documentation appended to SKILL.md
The adapter SHALL append a `## Tools` section to each generated SKILL.md documenting all tools available in the package.

#### Scenario: Tool documentation content
- **WHEN** a package declares a tool `generate` with description, input schema, and output schema
- **THEN** the generated `## Tools` section SHALL include for each tool:
  - Tool name as a `###` heading
  - Description text
  - Invocation example showing `./tools/<name> '{"input": "..."}'`
  - Input schema summary (property names, types, required fields)
  - Output schema summary (property names, types)

#### Scenario: Package with no tools
- **WHEN** a package declares no `tools` map
- **THEN** the adapter SHALL NOT append a `## Tools` section to the generated SKILL.md

### Requirement: Tool I/O transport convention
Tools SHALL accept input as a JSON string passed as the first positional argument and SHALL write output as a JSON object to stdout. Stderr is reserved for diagnostic/log output.

#### Scenario: Agent invokes a tool
- **WHEN** the agent calls `./tools/generate '{"spec": "openapi.yaml"}'`
- **THEN** the tool SHALL parse the JSON argument, execute its logic, and write a JSON result to stdout

#### Scenario: Tool error output
- **WHEN** a tool encounters an error during execution
- **THEN** the tool SHALL write a JSON object to stdout with a `status: "error"` field and a `message` field
- **THEN** the tool MAY write diagnostic information to stderr

### Requirement: Files include copying
At `skill build` time, the adapter SHALL copy all files matched by the `files` include globs into the generated skill directory, preserving relative directory structure.

#### Scenario: Logic files copied
- **WHEN** a manifest declares `files: ["logic/**"]` and the `logic/` directory contains `tools/generate.ts` and `templates/test.hbs`
- **THEN** the adapter output SHALL contain `logic/tools/generate.ts` and `logic/templates/test.hbs` at the same relative paths

#### Scenario: Multiple include patterns
- **WHEN** a manifest declares `files: ["content/**", "logic/**", "schemas/*.json"]`
- **THEN** the adapter SHALL copy all files matching any of the three patterns

### Requirement: Wrapper scripts reference correct relative paths
Wrapper scripts SHALL use relative paths from their location (`tools/`) to the actual entrypoint in the copied file tree, so they work regardless of where the skill directory is installed.

#### Scenario: Wrapper resolves entrypoint
- **WHEN** the generated directory structure is `.claude/skills/my-skill/tools/generate` and the entrypoint is at `.claude/skills/my-skill/logic/tools/generate.ts`
- **THEN** the wrapper SHALL resolve the entrypoint via `$(dirname "$0")/../logic/tools/generate.ts`
