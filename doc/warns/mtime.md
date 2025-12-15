# `mtime`

Warns when `mtime` is enabled.

`mtime` causes `lun` to skip files that have identical metadata to the last time
`lun` was run. This can significantly speed things up and is enabled by default.
However, in certain pathological cases, it can result in not running tools when
they should be run (e.g., if the system time was changed).

Default level: `allow`

In groups:

- `pedantic`
