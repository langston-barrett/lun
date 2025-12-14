# Configuration

The configuration file (`lun.toml` by default) is written in [TOML].

[TOML]: https://toml.io/en/

## Top-level fields

- `careful` (boolean, default: `false`): Include tool version in cache keys for more conservative caching.
- `cores` (integer, optional): Number of parallel jobs to run. If not specified, uses the number of CPU cores.
- `mtime` (boolean, default: `false`): Use file modification times.
- `ninja` (boolean, default: `false`): Enable or disable Ninja build file generation.
- `refs` (array of strings, default: `[]`): Git refs to compare against when determining which files to check.
- `tool` (array of tables): Array of tool configurations, see below.

### Warning configuration

- `allow` (array of strings, default: `[]`): Warning names to allow (suppress).
- `warn` (array of strings, default: `[]`): Warning names to warn about (print but continue).
- `deny` (array of strings, default: `[]`): Warning names to deny (print and exit with failure).

## `[[tool]]`

Each tool is defined in a `[[tool]]` table array.

- `name` (string, optional): Display name for the tool. If not specified, uses the command.
- `cmd` (string, required): Command to run for the tool.
- `files` (string, required): Glob pattern matching files that this tool should process.
- `granularity` (string, default: `"individual"`): How files are passed to the tool:

  - `"individual"`: One file per invocation
  - `"batch"`: All files in one invocation

- `configs` (array of strings, default: `[]`): Paths to configuration files that affect tool behavior. Changes to these files invalidate the cache.
- `check` (string, optional): Command to run in check-only mode (no modifications). If not specified, uses `cmd`.
- `fix` (string, optional): Command to run to automatically fix issues (see `--fix`).
- `formatter` (boolean, default: `false`): Whether this tool is a formatter (modifies files) rather than a linter (only checks).
