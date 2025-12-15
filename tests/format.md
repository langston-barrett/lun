# Formatter test

Test that formatters run.

## Scenario 1

### Config

```toml
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
granularity = "individual"
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
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
granularity = "individual"
```

### Output

```sh

```
