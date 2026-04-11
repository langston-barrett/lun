# `args = "none"` with no needed files

Test that commands with `args = "none"` should not run when there are no needed
files (e.g., when all files are cached).

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.toml"]
args = "none"
```

### `dummy.txt`

```
dummy
```

## Command

Cache is empty, so the command should execute.

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/1] echo (1/1)
[1/1] 1 file linted
```

## Command

No files have changed, so there are no "needed" files. The command should not
run.

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[0/0] 0 files linted
```