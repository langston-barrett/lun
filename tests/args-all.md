# `args = "all"`

Test that `args = "all"` passes all matching files in a single invocation.

This is useful for tools that need to see all files at once (e.g., `tagref` for
cross-file reference checking). Files are kept together even with multiple cores.

## Scenario 1

### Config

```toml
cores = 4

[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "all"
```

### Files

- `file1.txt`: `A`
- `file2.txt`: `A`
- `file3.txt`: `A`

### Output

```sh
echo file1.txt file2.txt file3.txt
```
