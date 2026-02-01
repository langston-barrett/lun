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
name = "fail"
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
[1/2] true (1/1)
[2/2] fail (1/1)
FAILED:
bash scripts/fail-slow.sh
[1/2] 1 error
```