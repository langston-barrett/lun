# Batching algorithm

Test that commands are distributed across batches based on the number of cores,
balanced by size.

## Scenario 1

### Config

```toml
cores = 3

[[linter]]
cmd = "lint --"
files = ["*.py"]
args = "many"
```

### Files

- `file1.py`: `AA`
- `file2.py`: `AAAA`
- `file3.py`: `AAA`
- `file4.py`: `A`
- `file5.py`: `AAAAAA`
- `file6.py`: `AA`

### Output

```sh
lint -- file5.py
lint -- file2.py file6.py
lint -- file1.py file3.py file4.py
```
