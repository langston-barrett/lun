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
[1/1] false lun.toml
Command failed:
false lun.toml
```
