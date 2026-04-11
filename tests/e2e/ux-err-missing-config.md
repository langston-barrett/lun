# Missing config file

Test the output when `lun.toml` doesn't exist.

## Files

### `file.txt`

```
hello
```

## Command

```sh
lun run
```

```
Config file not found. Hint: try `lun init`.
```