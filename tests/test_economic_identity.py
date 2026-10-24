from __future__ import annotations

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_manifest_uses_the_pallet_canonical_asset_id() -> None:
    manifest = json.loads(
        (ROOT / "deployment/genesis/x3-testnet-allocations.json").read_text(
            encoding="utf-8"
        )
    )
    pallet = (ROOT / "pallets/x3-coin/src/lib.rs").read_text(encoding="utf-8")
    match = re.search(r"pub const X3_ASSET_ID: u32 = (\d+);", pallet)

    assert match is not None
    assert manifest["token"]["asset_id"] == int(match.group(1))


def test_genesis_endowment_uses_the_canonical_twelve_decimal_unit() -> None:
    source = (ROOT / "node/src/chain_spec.rs").read_text(encoding="utf-8")

    assert "const X3: u128 = 1_000_000_000_000;" in source

