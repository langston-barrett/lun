# Invalid glob pattern

Test the output when a tool has an invalid glob pattern in `files`.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["[invalid"]
```

## Command

```sh
lun run
```

```
Invalid `files` glob `[invalid` for `echo`
```