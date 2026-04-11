# `args = "none"` with no needed files

Test that commands with `args = "none"` should not run when there are no needed
files (e.g., when all files are cached).

## Scenario 1

Cache is empty, so the command should execute.

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "none"
```

### Files

- `file.txt`: `A`

### Output

```sh
echo
```

## Scenario 2

No files have changed, so there are no "needed" files. The command should not
run.

### Config

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
args = "none"
```

### Output

```sh
```
