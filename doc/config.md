# Configuration

The configuration file (`lun.toml` by default) is written in [TOML].

[TOML]: https://toml.io/en/

## Top-level fields

- `careful` (boolean, default: `false`): Include tool version in cache keys for more conservative caching.
- `cores` (integer, optional): Number of parallel jobs to run. If not specified, uses the number of CPU cores.
- `mtime` (boolean, default: `false`): Use file modification times.
- `ninja` (boolean, default: `false`): Enable or disable Ninja build file generation.
- `refs` (array of strings, default: `[]`): Git refs to compare against when determining which files to check.
- `ignore` (array of strings, default: `[]`): Glob pattern(s) matching files that all tools should ignore.
- `linter` (array of tables): Array of linter configurations, see below.
- `formatter` (array of tables): Array of formatter configurations, see below.

### Warning configuration

- `allow` (array of strings, default: `[]`): Warning names to allow (suppress).
- `warn` (array of strings, default: `[]`): Warning names to warn about (print but continue).
- `deny` (array of strings, default: `[]`): Warning names to deny (print and exit with failure).

## `[[linter]]`

Each linter is defined in a `[[linter]]` table array.

- `name` (string, optional): Display name for the linter. If not specified, uses the command.
- `cmd` (string, required): Command to run for the linter.
- `files` (array of strings, required): Glob pattern(s) matching files that this linter should process.
- `ignore` (array of strings, default: `[]`): Glob pattern(s) matching files that this linter should ignore.
- `granularity` (string, default: `"individual"`): How files are passed to the linter:

  - `"individual"`: One file per invocation
  - `"batch"`: All files in one invocation

- `configs` (array of strings, default: `[]`): Paths to configuration files that affect linter behavior. Changes to these files invalidate the cache.
- `cd` (string, optional): Working directory for the linter.
- `fix` (string, optional): Command to run to automatically fix issues (see `--fix`). If not specified, uses `cmd`.

## `[[formatter]]`

Each formatter is defined in a `[[formatter]]` table array.

- `name` (string, optional): Display name for the formatter. If not specified, uses the command.
- `cmd` (string, required): Command to run for the formatter.
- `files` (array of strings, required): Glob pattern(s) matching files that this formatter should process.
- `ignore` (array of strings, default: `[]`): Glob pattern(s) matching files that this formatter should ignore.
- `granularity` (string, default: `"individual"`): How files are passed to the formatter:

  - `"individual"`: One file per invocation
  - `"batch"`: All files in one invocation

- `configs` (array of strings, default: `[]`): Paths to configuration files that affect formatter behavior. Changes to these files invalidate the cache.
- `cd` (string, optional): Working directory for the formatter.
- `check` (string, optional): Command to run in check-only mode (no modifications). If not specified, uses `cmd`.
