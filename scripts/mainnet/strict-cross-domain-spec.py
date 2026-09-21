#!/usr/bin/env python3
"""Flip a chain spec into the cross-domain proof posture live networks use.

`allowUnattestedCrossDomainProofs` is genesis state: `true` on dev/local (there
is no external chain to prove against there) and `false` everywhere a validator
can join, because a terminal refund released against a self-attested bundle is a
fund loss. The live cross-domain gates boot the dev chain, so they run the
permissive posture unless a spec with this one field flipped is supplied.

This script is deliberately hostile to the ways a security gate goes quiet:

  * a spec that does not carry the field at all is an error, not a no-op — the
    pallet may have been renamed or the field moved, and silently writing an
    unchanged file would make every "strict" run below it meaningless;
  * a source value that is not exactly `true` is an error, since the script
    exists to turn one posture into the other and cannot tell a typo from a
    policy;
  * the written file is read back and checked, because the failure mode this
    guards against is a gate that reports a posture it never applied.

Usage: strict-cross-domain-spec.py <source-spec.json> <dest-spec.json>
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

PALLET_KEY = "x3SettlementEngine"
FIELD_KEY = "allowUnattestedCrossDomainProofs"


def load_spec(path: pathlib.Path) -> tuple[dict, dict]:
    try:
        spec = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        sys.exit(f"error: cannot read {path}: {error}")
    try:
        return spec, spec["genesis"]["runtimeGenesis"]["config"]
    except (KeyError, TypeError):
        sys.exit(f"error: {path} has no genesis.runtimeGenesis.config")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("dest", type=pathlib.Path)
    args = parser.parse_args()

    spec, config = load_spec(args.source)
    pallet = config.get(PALLET_KEY)
    if not isinstance(pallet, dict) or FIELD_KEY not in pallet:
        sys.exit(
            f"error: {args.source} has no {PALLET_KEY}.{FIELD_KEY}. Refusing to write a "
            "spec that would leave the run permissive while reporting it as strict."
        )
    if pallet[FIELD_KEY] is not True:
        sys.exit(
            f"error: {PALLET_KEY}.{FIELD_KEY} is {pallet[FIELD_KEY]!r} in {args.source}, "
            "expected true (the dev value this script flips)"
        )

    pallet[FIELD_KEY] = False
    args.dest.write_text(json.dumps(spec))
    _, written_config = load_spec(args.dest)
    written = written_config.get(PALLET_KEY, {}).get(FIELD_KEY)
    if written is not False:
        sys.exit(f"error: {args.dest} still reports {PALLET_KEY}.{FIELD_KEY} = {written!r}")

    print(f"strict spec: {args.dest} ({PALLET_KEY}.{FIELD_KEY} = false)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
