# Invalid TOML syntax

Test the output when `lun.toml` has invalid TOML syntax.

## Files

### `lun.toml`

```toml
[[linter]
name = "echo"
```

## Command

```sh
lun run
```

```
Failed to parse config file: <TEMP>/lun.toml
```