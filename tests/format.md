# Formatter test

Test that formatters run.

## Scenario 1

### Config

```toml
[[tool]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
granularity = "individual"
formatter = true
```

### Files

- `file.py`: 8b

### Output

```sh
fmt -- file.py
```

## Scenario 2

### Config

```toml
[[tool]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
granularity = "individual"
formatter = true
```

### Output

```sh

```
