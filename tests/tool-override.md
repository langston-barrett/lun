# `[[tool]]` overriding behavior

Test that `[[tool]]` can use known tools and override specific fields.
Each scenario uses different file patterns to avoid cache interactions.

## Scenario 1

Use a known tool with default settings.

### Config

```toml
[[tool]]
name = "shellcheck"
configs = []
```

### Files

- `file.sh`: `A`

### Flags

```
--color never run
```

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

- `file.bash`: `A`
- `file.sh`: `A`

### Flags

```
--color never run
```

### Output

```sh
shellcheck --color=never -- file.bash
```
