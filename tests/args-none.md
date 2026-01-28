# `args = "none"`

Test that `args = "none"` doesn't pass files on the command line.

This is useful for tools that discover files themselves (e.g., `cargo clippy`).

## Scenario 1

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "none"
```

### Files

- `file1.txt`: 8b
- `file2.txt`: 8b

### Output

```sh
echo
```
