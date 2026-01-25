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
run
```

```
[0/?] Collecting files
[1/1] echo <TEMP>/really/ridiculously/long/path-name-with-many-components.txt
[1/1] 1 file linted
```