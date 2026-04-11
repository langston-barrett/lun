# no_default_ignores test

Test that image files are ignored by default, and that `no_default_ignores` disables this.

## Scenario 1

By default, image files (jpg, png, svg) are ignored.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*"]
args = "many"
```

### Files

- `file.py`: `A`
- `image.jpg`: `A`
- `image.png`: `A`
- `icon.svg`: `A`

### Output

```sh
lint -- file.py
```

## Scenario 2

With `no_default_ignores = true`, image files are linted.
Note: file.py is cached from scenario 1, so only image files appear.

### Config

```toml
no_default_ignores = true

[[linter]]
cmd = "lint --"
files = ["*"]
args = "many"
```

### Output

```sh
lint -- image.jpg image.png icon.svg
```
