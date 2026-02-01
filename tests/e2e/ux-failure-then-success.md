# Failure then success

Test that successes are not reported after failures.

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
run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/2] false (1/1)
FAILED:
false
[0/2] 1 error
```