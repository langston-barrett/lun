#!/usr/bin/env bash

set -euo pipefail

ver=0.14.8

cat > lun.toml <<EOF
[[tool]]
cmd = "rustfmt --check --"
files = "crates/**/*.rs"
granularity = "individual"
EOF

git checkout "${ver}"
# use whatever we've got
rm -f rust-toolchain.toml
# this file is problematic for some reason
rm -f crates/ruff_formatter/shared_traits.rs
cargo fmt
cargo fmt --check

do_time() { echo -- "${@}"; time -- "${@}"; }

# ---------------- Clean checkout

lun clean
lun run --dry-run --ninja --no-batch
do_time ninja -f .lun/build.ninja

lun clean
do_time lun run --no-batch

lun clean
do_time lun run

# ---------------- Checkout old commit

lun clean
lun run --ninja
# remove just the non-ninja cache to act like ninja would
for f in .lun/*; do
  if [[ $f == *ninja* ]]; then continue; fi
  rm -rf $f
done
git checkout 0.7.0
git checkout "${ver}"
lun run --dry-run --ninja --no-batch
do_time ninja -f .lun/build.ninja

lun clean
lun run
git checkout 0.7.0
git checkout "${ver}"
do_time lun run
