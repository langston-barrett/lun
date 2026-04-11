# `--color`

Test that `--color` replaces `{{color}}`.

## Scenario 1

### Config

```toml
[[linter]]
cmd = "lint --color {{color}} --"
files = ["*.py"]
args = "many"
```

### Files

- `file.py`: `A`

### Flags

```
--color always run
```

### Output

```sh
lint --color always -- file.py
```

## Scenario 2

### Config

```toml
[[linter]]
cmd = "lint --color {{color}} --"
files = ["*.py"]
args = "many"
```

### Files

- `file2.py`: `A`

### Flags

```
--color never run
```

### Output

```sh
lint --color never -- file2.py
```
