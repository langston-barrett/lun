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
[1/1] bogus <TEMP>/lun.toml
Command failed:
bogus <TEMP>/lun.toml
```