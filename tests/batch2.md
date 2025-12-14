# Batching algorithm

Test that commands are distributed across batches based on the number of cores,
balanced by size.

## Scenario 1

### Config

```toml
cores = 2

[[tool]]
cmd = "lint --"
files = "*.py"
granularity = "individual"
```

### Files

- `file1.py`: 100b
- `file2.py`: 200b
- `file3.py`: 150b
- `file4.py`: 50b
- `file5.py`: 300b
- `file6.py`: 100b

### Output

```sh
lint -- file1.py file4.py file5.py
lint -- file2.py file3.py file6.py
```
