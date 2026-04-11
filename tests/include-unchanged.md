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

- `file1.txt`: `A`
- `file2.txt`: `A`

### Output

```sh
echo file1.txt file2.txt
```

## Scenario 2

Has a new file, so needs to be rerun.

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

- `file1.txt`: `A`
- `file2.txt`: `A`
- `file3.txt`: `A`

### Output

```sh
echo file1.txt file2.txt file3.txt
```

## Scenario 3

No files have changed, so there are no "needed" files. The command should not
run.

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

- `file1.txt`: `A`
- `file2.txt`: `A`
- `file3.txt`: `A`

### Output

```sh
```
