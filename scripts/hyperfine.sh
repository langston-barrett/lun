#!/usr/bin/env bash

set -euo pipefail

tmp=$(mktemp)
cat > "${tmp}" <<EOF
[[linter]]
cmd = "true"
files = "*.rs"
[[linter]]
cmd = "true"
files = "*.md"
[[linter]]
cmd = "true"
files = "*.yml"
[[linter]]
cmd = "true"
files = "*.toml"
EOF

cargo build --profile=profiling --bin=lun
hyperfine --show-output --prepare 'rm -rf .lun' "./target/profiling/lun --config=${tmp} run"
