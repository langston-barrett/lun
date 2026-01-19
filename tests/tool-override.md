# `[[tool]]` overriding behavior

Test that `[[tool]]` can use known tools and override specific fields.
Each scenario uses different file sizes to avoid cache interactions.

## Scenario 1

Use a known tool with default settings. Using `mdlynx` since it has no config files.

### Config

```toml
[[tool]]
name = "mdlynx"
```

### Files

- `file.md`: 10b

### Output

```sh
mdlynx -- file.md
```

## Scenario 2

Override the `files` pattern to match a different extension.

### Config

```toml
[[tool]]
name = "mdlynx"
files = ["*.txt"]
```

### Files

- `file.md`: 20b
- `file.txt`: 20b

### Output

```sh
mdlynx -- file.txt
```
