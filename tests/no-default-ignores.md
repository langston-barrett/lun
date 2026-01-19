# no_default_ignores test

Test that image files are ignored by default, and that `no_default_ignores` disables this.

## Scenario 1

By default, image files (jpg, png, svg) are ignored.

### Config

```toml
[[linter]]
cmd = "lint --"
files = ["*"]
granularity = "individual"
```

### Files

- `file.py`: 100b
- `image.jpg`: 200b
- `image.png`: 150b
- `icon.svg`: 50b

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
granularity = "individual"
```

### Output

```sh
lint -- image.jpg image.png icon.svg
```
