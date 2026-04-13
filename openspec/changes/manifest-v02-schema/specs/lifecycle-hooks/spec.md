## ADDED Requirements

### Requirement: Hook declaration in manifest
The manifest SHALL support a top-level `hooks` map with the following optional keys: `onInstall`, `onActivate`, `onUninstall`. Each value SHALL be a string path to a shell script relative to the manifest directory.

#### Scenario: Valid hook declaration
- **WHEN** a manifest contains `hooks: { onInstall: "logic/hooks/setup.sh" }`
- **THEN** the parser SHALL accept the manifest and store the hook path

#### Scenario: Hook file not found
- **WHEN** a manifest declares a hook pointing to a file that does not exist on disk
- **THEN** validation SHALL fail with an error identifying the missing hook script

#### Scenario: No hooks declared
- **WHEN** a manifest omits the `hooks` field entirely
- **THEN** the parser SHALL accept the manifest with no lifecycle hooks

### Requirement: onInstall hook execution
The system SHALL execute the `onInstall` hook script after a skill package is installed via `skill install`. The hook SHALL run once per package, not per skill within the package.

#### Scenario: Successful onInstall
- **WHEN** `skill install` completes successfully and the manifest declares `hooks.onInstall`
- **THEN** the CLI SHALL execute the hook script with the package's installed directory as the working directory
- **THEN** the CLI SHALL report hook execution status (success or failure) to the user

#### Scenario: onInstall hook failure
- **WHEN** the `onInstall` hook script exits with a non-zero status
- **THEN** the CLI SHALL report the failure and the hook's stderr output
- **THEN** the CLI SHALL warn the user that the package may not function correctly but SHALL NOT roll back the installation

### Requirement: onActivate hook execution
The system SHALL execute the `onActivate` hook script after a skill package is activated via `skill activate`. The hook SHALL run once per activation, not per skill.

#### Scenario: Successful onActivate
- **WHEN** `skill activate` completes and the manifest declares `hooks.onActivate`
- **THEN** the CLI SHALL execute the hook script with the package's installed directory as the working directory

#### Scenario: onActivate verifies runtime availability
- **WHEN** an `onActivate` hook script checks for a required runtime (e.g., `node --version`) and the runtime is not available
- **THEN** the hook script exits non-zero and the CLI reports the failure to the user

### Requirement: onUninstall hook execution
The system SHALL execute the `onUninstall` hook script before a skill package is removed via `skill uninstall`. The hook SHALL run before any files are deleted.

#### Scenario: Cleanup before uninstall
- **WHEN** `skill uninstall` is invoked and the manifest declares `hooks.onUninstall`
- **THEN** the CLI SHALL execute the hook script before removing the package directory
- **THEN** if the hook fails, the CLI SHALL warn but proceed with uninstallation

### Requirement: Hook scripts are bundled by the adapter
Hook scripts referenced in the manifest SHALL be included in the adapter output. The adapter SHALL copy them into the generated skill directory alongside other included files.

#### Scenario: Hook script present in build output
- **WHEN** `skill build` runs on a package with `hooks.onInstall: "logic/hooks/setup.sh"`
- **THEN** the file SHALL be present in the generated output directory at its relative path
