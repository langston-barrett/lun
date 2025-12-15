# Changelog

<!-- https://keepachangelog.com/en/1.0.0/ -->

## next

- **BREAKING**: Split `[[tool]]` into `[[linter]]` and `[[formatter]]` arrays.
- Add `taplo` to the known tools

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
