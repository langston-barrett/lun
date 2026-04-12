# `--staged` flag

Test that `--staged` only runs on files staged in git.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.txt"]
```

### `a.txt`

```
a
```

### `b.txt`

```
b
```

### `c.txt`

```
c
```

## Setup

```sh
git init
git add lun.toml a.txt b.txt
```

## Command

With `--staged`, only staged files (`a.txt`, `b.txt`) should be processed.

```sh
lun run --staged --fresh
```

```
[0/?] Collecting files
[1/3] Planning
[2/3] Planning
[3/3] Planning
[1/1] echo (1/1)
[1/1] 2 files linted
```

## Command

With both `--staged` and `--only-files`, only files matching both filters should be
processed. `a.txt` is staged but doesn't match the glob. `c.txt` matches the glob but
isn't staged. Only `b.txt` satisfies both.

```sh
lun run --staged --only-files=**/b.txt --only-files=**/c.txt --fresh
```

```
[0/?] Collecting files
[1/1] Planning
[1/1] echo (1/1)
[1/1] 1 file linted
```

