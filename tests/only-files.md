# `--only-files`

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

- `file1.py`: `AA`
- `file2.py`: `AAAA`
- `different.py`: `AAA`
- `file4.py`: `A`

### Flags

```sh
run --only-files=file*.py
```

### Output

```sh
lint -- file2.py
lint -- file1.py file4.py
```
