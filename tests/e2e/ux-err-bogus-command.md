# Failure

Test the output when the tool executable can't be found.

## Files

### `lun.toml`

```toml
[[linter]]
name = "bogus"
cmd = "bogus"
files = ["*.toml"]
```

## Command

```sh
run
```

```
[0/?] Collecting files
[1/1] Planning
[1/1] bogus (1/1)
Failed to execute command: bogus <TEMP>/lun.toml
```