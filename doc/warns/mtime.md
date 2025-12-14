# `mtime`

Warns when `mtime` is set on CLI or config file.

`mtime` causes `lun` to skip files that have not been modified since it was
last run. This can significantly speed things up and is enabled by default.
However, in certain cases, it can result in not running tools when they should
be run (e.g., if the system time was changed). For more information, see [mtime
comparison considered harmful](https://apenwarr.ca/log/20181113).

Default level: `allow`

In groups:

- `pedantic`
