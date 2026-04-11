# --skip-files flag test

Test that --skip-files skips files matching the pattern.

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
- `different.py`: `A`
- `file4.py`: `A`

### Flags

```sh
run --skip-files=file*.py
```

### Output

```sh
lint -- different.py
```
