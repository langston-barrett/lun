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
git commit -m "Initial commit"
```

## Command

Without `--vcs`, both tracked and untracked files should be processed.

```sh
run --fresh
```

```
[0/?] Collecting files
[1/29] Planning
[2/29] Planning
[3/29] Planning
[4/29] Planning
[5/29] Planning
[6/29] Planning
[7/29] Planning
[8/29] Planning
[9/29] Planning
[10/29] Planning
[11/29] Planning
[12/29] Planning
[13/29] Planning
[14/29] Planning
[15/29] Planning
[16/29] Planning
[17/29] Planning
[18/29] Planning
[19/29] Planning
[20/29] Planning
[21/29] Planning
[22/29] Planning
[23/29] Planning
[24/29] Planning
[25/29] Planning
[26/29] Planning
[27/29] Planning
[28/29] Planning
[29/29] Planning
[1/1] echo <TEMP>/untracked.txt <TEMP>/tracked.txt
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
[1/1] echo <TEMP>/tracked.txt
[1/1] 1 file linted
```
