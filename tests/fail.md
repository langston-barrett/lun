# Running twice with a failure

When a command fails, it needs to be run again next time.

## Scenario 1

### Config

```toml
cores = 2

[[linter]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
```

### Files

- `file.py`: 8b
- `file2.py`: 8b

### Output

```sh
lint -- file.py
lint -- file2.py
```

### Fail

```sh
lint -- file.py
```

## Scenario 2

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
```

### Output

```sh
lint -- file.py
```
