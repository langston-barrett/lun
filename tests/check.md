# `--check` mode

Test that formatters use the `check` command when `--check` is passed.

## Scenario 1

Run formatter normally (uses `cmd`).

### Config

```toml
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
args = "many"
```

### Files

- `file.py`: 8b

### Output

```sh
fmt -- file.py
```

## Scenario 2

Run formatter with `--check` (uses `check` command).

### Config

```toml
[[formatter]]
cmd = "fmt --"
check = "fmt --check --"
files = ["*.py"]
args = "many"
```

### Flags

```sh
run --check
```

### Output

```sh
fmt --check -- file.py
```

## Scenario 3

Formatter without `check` field falls back to `cmd` when `--check` is passed.

### Config

```toml
[[formatter]]
cmd = "fmt --"
files = ["*.rs"]
args = "many"
```

### Files

- `file.rs`: 8b

### Flags

```sh
run --check
```

### Output

```sh
fmt -- file.rs
```
