# `.gitignore`

Test `.gitignore` handling.

## Files

### `lun.toml`

```toml
[[linter]]
name = "echo"
cmd = "echo"
files = ["*.py"]
```

### `.git/HEAD`

The `ignore` crate only uses `.gitignore` if a `.git/` directory is present.

```
```

### `.gitignore`

```
ignored/
*.pyc
temp_*.py
```

### `main.py`

```
```

### `ignored/module.py`

```
```

### `temp_script.py`

```
```

### `dir/cache.pyc`

```
```

### `ignored/nested/deep.py`

```python
```

## Command

```sh
run
```

```
[0/?] Collecting files
[1/3] Planning
[2/3] Planning
[3/3] Planning
[1/1] echo <TEMP>/main.py
[1/1] 1 file linted
```