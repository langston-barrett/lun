# Global ignore

Test that global ignore patterns work correctly, including with `./` prefix.

## Files

### `lun.toml`

```toml
ignore = ["./.claude/skills/skill-creator/**/*"]

[[linter]]
name = "echo"
cmd = "echo"
files = ["*.py"]
```

### `main.py`

```python
print("hello")
```

### `.claude/skills/skill-creator/main.py`

```python
print("should be ignored")
```

### `.claude/skills/skill-creator/nested/deep.py`

```python
print("also ignored")
```

## Command

```sh
run
```

```
[0/?] Collecting files
[1/4] Planning
[2/4] Planning
[3/4] Planning
[4/4] Planning
[1/1] echo (1 / 1)
[1/1] 1 file linted
```