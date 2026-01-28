# `args = "many"`

Test that `args = "many"` passes multiple files per invocation (the default).

This is the most common case, allowing tools to process multiple files efficiently
with parallelism when multiple cores are available.

## Scenario 1

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "many"
```

### Files

- `file1.txt`: 8b
- `file2.txt`: 8b
- `file3.txt`: 8b

### Output

```sh
echo file1.txt file2.txt file3.txt
```
