# Configuration

The configuration file (`lun.toml` by default) is written in [TOML].

[TOML]: https://toml.io/en/

## Top-level fields

- `careful` (boolean, default: `false`): Include tool version in cache keys for more conservative caching.
- `cache_size` (integer, optional): Maximum cache size in bytes. Defaults to 1.25 MiB.
- `cores` (integer, optional): Number of parallel jobs to run. If not specified, uses the number of CPU cores.
- `mtime` (boolean, default: `true`): Use file modification times (see [Caching](cache.md)).
- `ninja` (boolean, default: `false`): Enable or disable Ninja build file generation.
- `no_default_ignores` (boolean, default: `false`): Disable default ignore patterns for image files (JPG, JPEG, PNG, SVG).
- `refs` (array of strings, default: `[]`): Git refs to compare against when determining which files to check.
- `ignore` (array of strings, default: `[]`): Glob pattern(s) matching files that all tools should ignore. These patterns are combined with the default ignore patterns unless `no_default_ignores` is set.
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

  - `"individual"`: Any number of files per invocation, passed on the command line
  - `"batch"`: All files in one invocation, not passed on the command line

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

  - `"individual"`: Any number of files per invocation, passed on the command line
  - `"batch"`: All files in one invocation, not passed on the command line

- `configs` (array of strings, default: `[]`): Paths to configuration files that affect formatter behavior. Changes to these files invalidate the cache.
- `cd` (string, optional): Working directory for the formatter.
- `check` (string, optional): Command to run in check-only mode (no modifications). If not specified, uses `cmd`.

## `[[tool]]`

The `[[tool]]` table array provides a shorthand for configuring known tools.
Instead of specifying all fields manually, you can reference a tool by name and
optionally override specific fields.

```toml
[[tool]]
name = "cargo clippy"
```

This is equivalent to writing out the full `[[linter]]` configuration for `cargo
clippy` with all its default settings.

You can override specific fields while keeping the defaults for everything else:

```toml
[[tool]]
name = "cargo clippy"
ignore = ["generated/*.rs"]
configs = ["Cargo.toml", "clippy.toml"]
```

Available fields (all optional except `name`):

- `name` (string, required): Name of the known tool (e.g., `"cargo clippy"`, `"ruff check"`, `"cargo fmt"`).
- `cmd` (string, optional): Override the command to run.
- `files` (array of strings, optional): Override the file patterns.
- `ignore` (array of strings, optional): Override the ignore patterns.
- `granularity` (string, optional): Override the granularity (`"individual"` or `"batch"`).
- `configs` (array of strings, optional): Override the configuration file paths.
- `cd` (string, optional): Override the working directory.
- `fix` (string, optional): Override the fix command (for linters).
- `check` (string, optional): Override the check command (for formatters).

### Known tools

The following tools are recognized by name:

**Linters:**

- `cargo clippy`
- `hlint`
- `mdlynx`
- `mypy`
- `ruff check`
- `shellcheck`
- `tagref`
- `ttlint`
- `ty`
- `typos`
- `zizmor`

**Formatters:**

- `cargo fmt`
- `ruff format`
- `taplo`
