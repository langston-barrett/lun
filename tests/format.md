# Formatter test

Test that formatters run.

## Scenario 1

### Config

```toml
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
args = "many"
```

### Files

- `file.py`: `A`

### Output

```sh
fmt -- file.py
```

## Scenario 2

Same file, same config. Cache hit from scenario 1.

### Config

```toml
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
args = "many"
```

### Output

```sh
```
