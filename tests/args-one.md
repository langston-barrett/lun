# `args = "one"`

Test that `args = "one"` passes exactly one file per invocation.

This is useful for tools that can only process one file at a time.

## Scenario 1

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "one"
```

### Files

- `file1.txt`: `A`
- `file2.txt`: `A`
- `file3.txt`: `A`

### Output

```sh
echo file1.txt
echo file2.txt
echo file3.txt
```
