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
[1/1] Planning
[1/1] echo (1 / 1)
[1/1] 1 file linted
```