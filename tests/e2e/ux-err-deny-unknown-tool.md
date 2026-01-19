# `--deny=unknown-tool`

Test the output when `--deny=unknown-tool` is used with a bad `--only-tool`.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
```

### `file.txt`

```
hello
```

## Command

```sh
--deny=unknown-tool run --only-tool=nonexistent
```

```
found unknown tool names and --deny=unknown-tool
```
