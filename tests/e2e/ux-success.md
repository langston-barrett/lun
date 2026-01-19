# Success

Test the output on success.

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
run
```

```
[0/?] Collecting files
[1/1] echo lun.toml
[1/1] 1 file linted
```
