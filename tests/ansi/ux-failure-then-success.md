# Failure then success

Test that successes are not reported after failures with ANSI colors enabled.

## Files

### `lun.toml`

```toml
[[linter]]
cmd = "false"
args = "none"
files = ["*.toml"]

[[linter]]
args = "none"
cmd = "sleep 1"
files = ["*.toml"]
```

## Command

```sh
lun run
```

```
\r[0/?] Collecting files
\r[1/2] Planning
\r[1/2] false (1/1)
red[FAILED]:
false

\rred[[0/2]] 1 error
```
