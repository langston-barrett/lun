# `--quiet`

Test the output with `--quiet`.

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
lun -q run
```

TODO(#145): This should not be the same as `ux-success.md`.

```
[0/?] Collecting files
[1/1] Planning
[1/1] echo (1/1)
[1/1] 1 file linted
```