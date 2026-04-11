# No-op (no files match)

Test the output when no files match any tool.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.py"]
```

### `file.txt`

```
hello
```

## Command

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[0/0] 0 files linted
```