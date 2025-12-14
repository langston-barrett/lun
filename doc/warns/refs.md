# `refs`

Warns when `refs` is used on CLI or config file.

Like `mtime`, `refs` causes `lun` to skip certain files. However, it implicitly
assumes that the exact same version and configuration of the linter was run on
the refs in question. In certain cases, it can result in not running tools when
they should be run. `refs` is not enabled by default.

Default level: `allow`

In groups:

- `all`
- `pedantic`
