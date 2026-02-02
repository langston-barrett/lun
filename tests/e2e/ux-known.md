# Known tools

Test the output of `lun known`.

## Command

```sh
known
```

```
[[linter]]
name = "bash -n"
cmd = "bash -n --"
files = ["*.sh"]

[[linter]]
name = "cargo clippy"
cmd = "cargo clippy --color={{color}} --all-targets -- --deny warnings"
files = ["*.rs"]
args = "none"
configs = ["Cargo.toml"]
fix = "cargo clippy --color={{color}} --allow-dirty --fix"

[[linter]]
name = "cargo test"
cmd = "cargo test --color={{color}}"
files = ["*.rs"]
args = "none"
configs = ["Cargo.toml"]

[[linter]]
cmd = "hlint --"
files = ["*.hs"]
configs = [
    ".hlint.yml",
    ".hlint.yaml",
]

[[linter]]
name = "jq null"
cmd = "jq null --"
files = ["*.json"]

[[linter]]
name = "make -n"
cmd = "make -n"
files = [
    "**/Makefile",
    "*.mk",
]
args = "none"

[[linter]]
name = "mdlynx"
cmd = "mdlynx --"
files = ["*.md"]

[[linter]]
name = "mypy"
cmd = "mypy --strict --"
files = ["*.py"]
configs = [
    "pyproject.toml",
    "mypy.ini",
    ".mypy.ini",
]

[[linter]]
name = "ruff check"
cmd = "ruff check --"
files = ["*.py"]
configs = [
    "pyproject.toml",
    "ruff.toml",
    ".ruff.toml",
]
fix = "ruff check --fix --"

[[linter]]
name = "shellcheck"
cmd = "shellcheck --color={{color}} --"
files = ["*.sh"]
configs = [".shellcheckrc"]

[[linter]]
name = "tagref"
cmd = "tagref check --"
files = ["*"]
args = "none"
include_unchanged = true

[[linter]]
name = "ttlint"
cmd = "ttlint --"
files = ["*"]
fix = "ttlint --fix --"

[[linter]]
name = "ty"
cmd = "ty check --"
files = ["*.py"]
args = "none"
configs = [
    "pyproject.toml",
    "ty.toml",
]

[[linter]]
name = "typos"
cmd = "typos --"
files = ["*.md"]
configs = [
    "typos.toml",
    "_typos.toml",
    ".typos.toml",
]
fix = "typos --write-changes --"

[[linter]]
name = "zizmor"
cmd = "zizmor --"
files = [".github/**/*.yml"]
configs = [
    "zizmor.yml",
    "zizmor.yaml",
]
fix = "zizmor --fix=safe --"

[[formatter]]
name = "cargo fmt"
cmd = "cargo fmt -- --color={{color}} --"
files = ["*.rs"]
args = "none"
configs = [
    "Cargo.toml",
    "rustfmt.toml",
    ".rustfmt.toml",
]
check = "cargo fmt -- --check --color={{color}} --"

[[formatter]]
name = "ruff format"
cmd = "ruff format --"
files = ["*.py"]
configs = [
    "ruff.toml",
    ".ruff.toml",
]
check = "ruff format --check --"

[[formatter]]
name = "taplo"
cmd = "taplo format --"
files = ["*.toml"]
check = "taplo format --check --"
```