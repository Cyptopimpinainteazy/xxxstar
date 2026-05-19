#!/usr/bin/env bash

set -euo pipefail

cd "${0%/*}/"
cd ..

./ci/jobs/build-and-test.sh
./ci/jobs/clippy.sh
./ci/jobs/rustfmt.sh

echo "----------------------------------------"
echo "All tests finished!"
