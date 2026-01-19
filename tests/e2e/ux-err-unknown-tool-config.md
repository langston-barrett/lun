# Unknown tool in [[tool]]

Test the output when an unknown tool name is used in the `[[tool]]` section.

## Files

### `lun.toml`

```toml
[[tool]]
name = "nonexistent-tool"
```

## Command

```sh
run
```

```
Unknown tool name in [[tool]]: nonexistent-tool
```
