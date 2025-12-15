# Caching

Lūn uses a cache to avoid re-running tools on files that haven't changed. By
default, the cache is stored in `.lun/cache` in the project root. `lun clean`
clears the cache. Cache entries older than 30 days are automatically deleted.

## Keys

There are two kinds of cache entry. They both include the:

- File path
- File metadata, including size, owner UID and GID, and permissions (mode)
- Tool command line
- Tool working directory, if specified
- Content of the tool configuration file(s), if specified
- Output of the tool's `--version` flag (if `--careful` is used)

`mtime` entries also include the file modification time.
Content entries also include the hash of the file content.
