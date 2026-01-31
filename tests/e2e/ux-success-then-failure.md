# Success then failure

Test that failures are reported after successes.

## Files

### `lun.toml`

```toml
[[linter]]
cmd = "true"
args = "none"
files = ["*.toml"]

[[linter]]
args = "none"
cmd = "bash scripts/fail-slow.sh"
files = ["*.toml"]
```

## Command

```sh
run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/2] true
[2/2] bash scripts/fail-slow.sh
Command failed:
bash scripts/fail-slow.sh
```