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
[1/6] Planning
[2/6] Planning
[3/6] Planning
[4/6] Planning
[5/6] Planning
[6/6] Planning
[1/2] false <TEMP>/file.a
Command failed:
false <TEMP>/file.a

[2/2] false <TEMP>/file.b
Command failed:
false <TEMP>/file.b
```