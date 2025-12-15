# `cd`

## Scenario 1

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
cd = "subdir"
```

### Files

- `subdir/file.py`: 8b

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
granularity = "individual"
cd = "subdir"
```

### Files

- `subdir/nested/file.py`: 8b

### Output

```sh
cd subdir && lint -- nested/file.py
```
