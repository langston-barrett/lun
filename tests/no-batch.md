# `--no-batch`

## Scenario 1

### Config

```toml
cores = 2

[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
```

### Files

- `file1.py`: `A`
- `file2.py`: `A`
- `file3.py`: `A`
- `file4.py`: `A`

### Flags

```sh
run --no-batch
```

### Output

```sh
lint -- file1.py
lint -- file2.py
lint -- file3.py
lint -- file4.py
```
