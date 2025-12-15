# `--no-batch`

## Scenario 1

### Config

```toml
cores = 2

[[linter]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
```

### Files

- `file1.py`: 100b
- `file2.py`: 200b
- `file3.py`: 150b
- `file4.py`: 50b

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
