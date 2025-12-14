# `careful`

Warns when `careful` is not set at CLI or config level.

`careful` causes `lun` to include tool versions in its cache keys. This can add
significant overhead, and so is disabled by default. However, this means that
`lun` might not re-run a linter after the tool itself has been upgraded.

Default level: `allow`

In groups:

- `pedantic`
