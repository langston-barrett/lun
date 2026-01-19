# Multiple failures with keep-going

Test that multiple failures are reported when using `--keep-going`.

## Files

### `lun.toml`

```toml
[[linter]]
name = "false1"
cmd = "false"
files = ["*.a"]

[[linter]]
name = "false2"
cmd = "false"
files = ["*.b"]
```

### `file.a`

```
a
```

### `file.b`

```
b
```

## Command

```sh
run --keep-going
```

TODO(#81): Fix 0 files found

```
[0/?] Collecting files
[0/0] 0 files linted
```
