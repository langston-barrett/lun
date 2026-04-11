# `cd`

## Scenario 1

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
cd = "subdir"
```

### Files

- `subdir/file.py`: `A`

### Output

```sh
cd subdir && lint -- file.py
```

## Scenario 2

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
cd = "subdir"
```

### Files

- `subdir/nested/file.py`: `A`

### Output

```sh
cd subdir && lint -- nested/file.py
```
