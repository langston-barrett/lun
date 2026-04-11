# Failure

Test the output on linter failure with ANSI colors enabled.

## Files

### `lun.toml`

```toml
[[linter]]
name = "false"
cmd = "false"
files = ["*.toml"]
```

## Command

```sh
lun run
```

```
\r[0/?] Collecting files
\r[1/1] Planning
\r[1/1] false (1/1)
red[FAILED]:
false <TEMP>/lun.toml

\rred[[0/1]] 1 error
```
