# `--vcs` flag

Test that `--vcs` only runs on VCS-tracked files.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
```

### `tracked.txt`

```
```

### `untracked.txt`

```
```

## Setup

```sh
git init
git add lun.toml tracked.txt
```

## Command

Without `--vcs`, both tracked and untracked files should be processed.

```sh
run --fresh
```

```
[0/?] Collecting files
[1/3] Planning
[2/3] Planning
[3/3] Planning
[1/1] echo (1 / 1)
[1/1] 2 files linted
```

## Command

With `--vcs`, only the tracked file should be processed.

```sh
run --vcs
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/1] echo (1 / 1)
[1/1] 1 file linted
```