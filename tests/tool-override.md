# `[[tool]]` overriding behavior

Test that `[[tool]]` can use known tools and override specific fields.
Each scenario uses different file sizes to avoid cache interactions.

## Scenario 1

Use a known tool with default settings.

### Config

```toml
[[tool]]
name = "shellcheck"
configs = []
```

### Files

- `file.sh`: 10b

### Output

```sh
shellcheck --color=never -- file.sh
```

## Scenario 2

Override the `files` pattern to match a different extension.

### Config

```toml
[[tool]]
name = "shellcheck"
files = ["*.bash"]
configs = []
```

### Files

- `file.bash`: 20b
- `file.sh`: 20b

### Output

```sh
shellcheck --color=never -- file.bash
```
