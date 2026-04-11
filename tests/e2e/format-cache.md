# Formatter caching

Test that after a formatter modifies a file, the post-modification content is
cached. On the next run the file should be a cache hit. When the original
violation is re-introduced, the formatter must run again.

## Files

### `lun.toml`

```toml
[[formatter]]
cmd = "sed -i s/bad/good/"
files = ["*.txt"]
```

### `test.txt`

```
bad
```

## Command

First run: formatter modifies the file.

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/1] sed -i s/bad/good/ (1/1)
[1/1] 1 file linted
```

## Command

Second run: file hasn't changed since the formatter ran, so it's cached.

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[0/0] 0 files linted
```

## Command

Re-introduce the violation.

```sh
printf 'bad\n' > test.txt
```

## Command

Third run: violation is back, formatter must run again.

```sh
lun run
```

```
[0/?] Collecting files
[1/2] Planning
[2/2] Planning
[1/1] sed -i s/bad/good/ (1/1)
[1/1] 1 file linted
```
