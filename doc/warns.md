# Warnings

Lūn supports various *warnings*. Each warning has a name and a default level.
The levels are:

- `allow`: Do not print a warning
- `warn`: Print a warning, but continue
- `deny`: Print a warning and exit with failure

Levels can be overridden on the command line with `--allow`/`-A`, `--warn`/`-W`,
or `--deny`/`-D`, or in the configuration file in the `allow`, `warn`, or `deny`
arrays.

`lun warns` lists the warnings, and `lun warns WARN` prints the documentation
for `WARN`.

## `careful`

{{#include warns/careful.md:2:}}

## `mtime`

{{#include warns/mtime.md:2:}}

## `no-files`

{{#include warns/no-files.md:2:}}

## `refs`

{{#include warns/refs.md:2:}}

## `unknown-tool`

{{#include warns/unknown-tool.md:2:}}

## `unknown-warning`

{{#include warns/unknown-warning.md:2:}}

## `unlisted-config`

{{#include warns/unlisted-config.md:2:}}

## `cache-full`

{{#include warns/cache-full.md:2:}}

## `cache-usage`

{{#include warns/cache-usage.md:2:}}

