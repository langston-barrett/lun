# Changing the linter command line

Test that changing the command line results in re-running the tool.

## Scenario 1

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
```

### Files

- `file.py`: 8b

### Output

```sh
lint -- file.py
```

## Scenario 2

### Config

```toml
[[linter]]
cmd = "lint --some-flag --"
files = ["*.py"]
granularity = "individual"
```

### Output

```sh
lint --some-flag -- file.py
```
