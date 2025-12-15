# --skip-files flag test

Test that --skip-files skips files matching the pattern.

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
- `different.py`: 150b
- `file4.py`: 50b

### Flags

```sh
run --skip-files=file*.py
```

### Output

```sh
lint -- different.py
```
