# Usage

It's as easy as:

- `lun init`
- Add or remove linters in `lun.toml`
- `lun run` (or `lun run --watch`)

## As a pre-commit hook

```sh
cat <<'EOF' > .git/hooks/pre-commit
#!/usr/bin/env bash
lun run --check --staged
EOF
chmod +x .git/hooks/pre-commit
```

## In GitHub Actions

Lūn provides a GitHub action. To use it, replace `SHA` by the commit of the
most recent release, and use:

```yaml
- uses: langston-barrett/lun/.github/actions/lun@SHA
```
