# Failure

Test the output on linter failure.

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
run
```

```
[0/?] Collecting files
[1/1] Planning
[1/1] false (1/1)
FAILED:
false <TEMP>/lun.toml
[0/1] 1 error
```