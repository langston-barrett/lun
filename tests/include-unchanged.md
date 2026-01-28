# `include_unchanged`

Test that `include_unchanged = true` includes all matching files, not just
changed ones.

## Scenario 1

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "all"
include_unchanged = true
```

### Files

- `file1.txt`: 8b
- `file2.txt`: 8b

### Output

```sh
echo file1.txt file2.txt
```

## Scenario 2

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "all"
include_unchanged = true
```

### Files

- `file1.txt`: 8b
- `file2.txt`: 8b

### Output

```sh
echo file1.txt file2.txt
```
