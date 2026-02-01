#!/usr/bin/env bash

# See doc/dev/issues.md

# shellcheck disable=SC2016

set -euo pipefail

l() {
   gh label create "${2}" --color "${1}" --description "${3}" --force
}

l 6699cc 'area/ci' 'CI'
l 6699cc 'area/cli' 'Command-line interface'
l 6699cc 'area/config' 'Configuration file'
l 6699cc 'area/docs' 'Documentation'
l 6699cc 'area/logging' 'Terminal output'
l 6699cc 'area/tests' 'Tests'
l 6699cc 'area/watch' '`run --watch`'

l ed9121 'status/needs discussion' 'Needs further discussion to proceed'
l ed9121 'status/needs repro' 'Needs reproducer'
l ed9121 'status/needs mcve' 'Has a repro, but needs a Minimal Complete and Verifiable Example'
l ed9121 'status/needs test' 'Has a MCVE, but needs it to be checked into the repo as a test'
l 008672 'status/has test' 'Has a test in the repo'

l 9400d3 'topic/performance' 'Relating to runtime performance'
l 9400d3 'topic/tech debt' 'Relating to technical debt'
l 9400d3 'topic/ux' 'Relating to the User eXperience'

l d73a4a 'type/bug' 'Something is not working as expected'
l cfd3d7 'type/duplicate' 'This issue or pull request already exists'
l a2eeef 'type/feature' 'New feature idea or request'
l d876e3 'type/question' 'A question from a user'
l 0ed7e6 'type/refactor' 'An intrinsic improvement without behavioral changes'

