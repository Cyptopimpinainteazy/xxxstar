#!/bin/bash
# Wrapper for zombienet: strips the x3-chain-node banner from stdout
# so that `build-spec` output is clean JSON
exec ./target/release/x3-chain-node "$@" 2>/dev/null | sed -n '/^{/,$ p'