from __future__ import annotations

import re
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def load_features(path: str) -> dict[str, object]:
    with (ROOT / path).open("rb") as handle:
        return tomllib.load(handle)["features"]


def test_mainnet_v1_is_one_explicit_runtime_surface() -> None:
    features = load_features("runtime/Cargo.toml")

    assert features["mainnet-v1"] == [
        "mainnet-rc1",
        "external-gateway",
        "native-real-vm-adapters",
    ]
    assert features["external-gateway"] == [
        "pallet-x3-cross-vm-router/external-gateway",
    ]


def test_mainnet_v1_uses_the_real_router_scope_without_testnet() -> None:
    router = load_features("pallets/x3-cross-vm-router/Cargo.toml")

    assert router["mainnet-v1"] == ["mainnet-rc1", "external-gateway"]
    assert "testnet" not in router["mainnet-v1"]


def test_node_exposes_the_same_mainnet_v1_surface() -> None:
    node = load_features("node/Cargo.toml")

    assert node["mainnet-v1"] == ["x3-chain-runtime/mainnet-v1"]
    assert "mainnet-rc1" not in node["mainnet-v1"]


def test_router_scope_guard_permits_only_the_explicit_mainnet_v1_exception() -> None:
    source = (ROOT / "pallets/x3-cross-vm-router/src/lib.rs").read_text(encoding="utf-8")

    assert re.search(
        r'#\[cfg\(\s*all\(\s*feature = "mainnet-rc1",\s*'
        r'feature = "external-gateway",\s*'
        r'not\(feature = "mainnet-v1"\)\s*\)\s*\)\]',
        source,
    )

