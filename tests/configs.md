# `configs` dependency tracking

Test that changing a config file invalidates the cache.

Uses `Cargo.toml` as a real config file that exists in the repo.

## Scenario 1

First run with config dependency.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
configs = ["Cargo.toml"]
```

### Files

- `file.py`: 8b

### Output

```sh
lint -- file.py
```

## Scenario 2

Second run with same config - should be cached (no output).

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
configs = ["Cargo.toml"]
```

### Output

```sh

```

## Scenario 3

Run with different config file - should invalidate cache and re-run.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
configs = ["Cargo.lock"]
```

### Output

```sh
lint -- file.py
```

## Scenario 4

Run with no config dependency - different stamp, should re-run.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
```

### Output

```sh
lint -- file.py
```

## Scenario 5

Run with multiple configs - should be different stamp from single config.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
configs = ["Cargo.toml", "Cargo.lock"]
```

### Output

```sh
lint -- file.py
```
