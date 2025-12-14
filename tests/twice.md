# Running twice with no changes

Test that when nothing has changed, commands are not rerun.

## Scenario 1

### Config

```toml
[[tool]]
cmd = "lint --"
files = "*.py"
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
[[tool]]
cmd = "lint --"
files = "*.py"
granularity = "individual"
```

### Output

```sh

```
