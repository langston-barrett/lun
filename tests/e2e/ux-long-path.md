# Long path name

Test the output when there is a really long path name.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
```

### `really/ridiculously/long/path-name-with-many-components.txt`

```
hello
```

## Command

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/1] echo (1/1)
[1/1] 1 file linted
```