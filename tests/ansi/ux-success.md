# Success

Test the output on success with ANSI colors enabled.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.toml"]
```

## Command

```sh
lun run
```

```
\r[0/?] Collecting files
\r[1/1] Planning
\r[1/1] echo (1/1)
\rgreen[[1/1]] 1 file linted
```
