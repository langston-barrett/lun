# Running twice with no changes

Test that when nothing has changed, commands are not rerun.

## Scenario 1

### Config

```toml
[[linter]]
cmd = "lint --"
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

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
```

### Output

```sh

```
