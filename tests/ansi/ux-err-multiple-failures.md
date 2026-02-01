# Multiple failures with keep-going

Test that multiple failures are reported when using `--keep-going` with ANSI colors enabled.

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
\r[0/?] Collecting files
\r[1/6] Planning
\r[1/2] false1 (1/1)
red[FAILED]:
false <TEMP>/file.a

red[FAILED]:
false <TEMP>/file.b

\rred[[2/2]] 2 errors
```
