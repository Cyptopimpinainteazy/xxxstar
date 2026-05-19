#!/usr/bin/env bash
set -euo pipefail

cd /home/lojak/Desktop/X3_ATOMIC_STAR/x3fronend
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
export PORT=4174
npm run server
