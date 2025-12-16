# Changelog

<!-- https://keepachangelog.com/en/1.0.0/ -->

## [0.5.0] - 2025-12-16

[0.5.0]: https://github.com/langston-barrett/lun/releases/tag/v0.5.0

- Add `--no-cache` flag to disable cache
- Add `--no-refs` flag to ignore any refs from CLI or config file
- Add `--fresh` flag equivalent to `--no-cache --no-refs`
- Add cache size limit (config: `cache_size`, cli: `--cache-size`)
- Add cache management commands (`cache {gc,rm,stats}`)

## [0.4.0] - 2025-12-15

[0.4.0]: https://github.com/langston-barrett/lun/releases/tag/v0.4.0

- Reduce mixing of output in logs
- Overhaul handling of file modification times. Enable `mtime` by default.

## [0.3.0] - 2025-12-15

[0.3.0]: https://github.com/langston-barrett/lun/releases/tag/v0.3.0

- **BREAKING**: Split `[[tool]]` into `[[linter]]` and `[[formatter]]` arrays.
- Add `taplo` to the known tools
- Add global `ignore` array
- Add working directory configuration to tools (`cd`)

## [0.2.0] - 2025-12-15

[0.2.0]: https://github.com/langston-barrett/lun/releases/tag/v0.2.0

- `tool.files` now requires a list of globs
- Add `tool.ignore` for ignoring a list of globs
- Add more flags and tools to `--init`
- Ignore mtimes if `lun.toml` has changed
- Fix GitHub Actions

## [0.1.1] - 2025-12-14

[0.1.1]: https://github.com/langston-barrett/lun/releases/tag/v0.1.1

- Fix publishing of release artifacts.

## [0.1.0] - 2025-12-14

[0.1.0]: https://github.com/langston-barrett/lun/releases/tag/v0.1.0

Initial release!
