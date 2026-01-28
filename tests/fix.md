# `--fix` mode

Test that linters use the `fix` command when `--fix` is passed.

## Scenario 1

Run linter normally (uses `cmd`).

### Config

```toml
[[linter]]
cmd = "lint --"
fix = "lint --fix --"
files = ["*.py"]
args = "many"
```

### Files

- `file.py`: 8b

### Output

```sh
lint -- file.py
```

## Scenario 2

Run linter with `--fix` (uses `fix` command).

### Config

```toml
[[linter]]
cmd = "lint --"
fix = "lint --fix --"
files = ["*.py"]
args = "many"
```

### Flags

```sh
run --fix
```

### Output

```sh
lint --fix -- file.py
```

## Scenario 3

Linter without `fix` field falls back to `cmd` when `--fix` is passed.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.rs"]
args = "many"
```

### Files

- `file.rs`: 8b

### Flags

```sh
run --fix
```

### Output

```sh
lint -- file.rs
```
