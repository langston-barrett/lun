# Caching

Lūn uses a cache to avoid re-running tools on files that haven't changed. By
default, the cache is stored in `.lun/cache` in the project root. `lun clean`
clears the cache. Cache entries older than 30 days are automatically deleted.

## Keys

There are two kinds of cache entry. They both include the following:

- File path
- File metadata, including size, owner UID and GID, and permissions (mode)
- Tool command line
- Tool working directory, if specified
- Content of the tool configuration file(s), if specified
- Output of the tool's `--version` flag (if `--careful` is used)

`mtime` entries also include the file modification time.
*Content* entries also include the hash of the file content.

## Caching strategy

For each (file, tool) pair, Lūn does the following:

- If `mtime` is enabled (it is by default, but can be disabled with `--no-mtime`
  or `mtime = false`), Lūn first checks if there is an `mtime` cache entry for
  the pair. If so, it skips the pair.
- Otherwise, Lūn checks for a content entry. If present, it skips the pair.
- Otherwise, Lūn runs the tool on the file (possibly in a batch with other
  files).
- If successful, it saves a content entry for the pair. If `mtime` is enabled,
  it also saves an `mtime` entry.

