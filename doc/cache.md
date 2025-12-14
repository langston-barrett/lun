# Caching

Lūn uses a cache to avoid re-running tools on files that haven't changed. By
default, the cache is stored in `.lun/cache` in the project root. `lun clean`
clears the cache. Cache entries older than 30 days are automatically deleted.

## Keys

A cache entry consists of the hash of the:

- File path
- File content
- File metadata, including size, owner UID and GID, and permissions (mode)
- Tool command line
- Content of the linter configuration file(s), if specified
- Output of the tool's `--version` flag (if `--careful` is used)
