#!/usr/bin/env bash
set -euo pipefail

pattern='TODO|FIXME|stub|mock|fake|placeholder|dummy|unimplemented!|todo!|panic!\("not implemented'
roots=(node runtime crates pallets bridges packages infra-structure)
existing_roots=()

for root in "${roots[@]}"; do
  if [[ -d "$root" ]]; then
    existing_roots+=("$root")
  fi
done

if [[ ${#existing_roots[@]} -eq 0 ]]; then
  printf 'no production roots found\n' >&2
  exit 2
fi

find -P "${existing_roots[@]}" \
  -type d \( \
    -name .git -o -name node_modules -o -name target -o -name dist -o \
    -name build -o -name generated -o -name tests -o -name test -o -name fuzz -o \
    -name '*.egg-info' -o -name __pycache__ \
  \) -prune -o \
  -type f \( \
    -name '*.rs' -o -name '*.py' -o -name '*.ts' -o -name '*.tsx' -o \
    -name '*.js' -o -name '*.jsx' -o -name '*.sol' -o -name '*.sh' -o \
    -name '*.toml' -o -name '*.yml' -o -name '*.yaml' \
  \) \
  ! -name 'mock.rs' ! -name 'benchmarking.rs' ! -name 'tests.rs' \
  ! -name '*test*.rs' ! -name '*test*.py' ! -name '*test*.ts' \
  ! -name '*test*.tsx' ! -name '*test*.js' ! -name '*test*.jsx' -print0 \
  | xargs -0 --no-run-if-empty grep -nEI "$pattern"
