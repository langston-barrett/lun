#!/usr/bin/env bash

set -euo pipefail

tmp=$(mktemp)
cat > "${tmp}" <<EOF
[[tool]]
cmd = "true"
files = "*.rs"
[[tool]]
cmd = "true"
files = "*.md"
[[tool]]
cmd = "true"
files = "*.yml"
[[tool]]
cmd = "true"
files = "*.toml"
EOF

cargo build --profile=profiling --bin=lun
hyperfine --show-output --prepare 'rm -rf .lun' "./target/profiling/lun --config=${tmp} run"
