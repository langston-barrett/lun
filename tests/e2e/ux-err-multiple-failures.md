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

```
[0/?] Collecting files
[1/2] false <TEMP>/file.a
Command failed:
false <TEMP>/file.b


[2/2] false <TEMP>/file.a
Command failed:
false <TEMP>/file.a
```
