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
        "pallet-x3-cross-vm-router/mainnet-v1",
    ]
    assert features["external-gateway"] == [
        "pallet-x3-cross-vm-router/external-gateway",
    ]


def test_runtime_forwards_the_router_scope_lock() -> None:
    """The pallet's scope guards are inert unless the runtime forwards the token.

    `pallet-x3-cross-vm-router`'s `mainnet-rc1` feature gates no code; its only
    effect is to arm the `compile_error!` scope guards. The runtime used to forward
    neither it nor `mainnet-v1`, so a `mainnet-v1` build resolved the pallet to
    `external-gateway, serde, std` — the audited external gateway switched on with
    every scope guard disarmed. Measured with
    `cargo tree -e features -i pallet-x3-cross-vm-router`, and the guard was
    confirmed live by compiling the pallet with `mainnet-rc1,external-gateway`,
    which fails with MAINNET SCOPE VIOLATION.
    """
    features = load_features("runtime/Cargo.toml")

    assert features["mainnet-rc1"] == ["pallet-x3-cross-vm-router/mainnet-rc1"]
    # `mainnet-v1` reaches the lock through the runtime's own `mainnet-rc1`, which
    # forwards it to the pallet; and it carries the pallet's explicit exception.
    assert "mainnet-rc1" in features["mainnet-v1"]
    assert "pallet-x3-cross-vm-router/mainnet-v1" in features["mainnet-v1"]


def test_mainnet_v1_uses_the_real_router_scope_without_testnet() -> None:
    router = load_features("pallets/x3-cross-vm-router/Cargo.toml")

    assert router["mainnet-v1"] == ["mainnet-rc1", "external-gateway"]
    assert "testnet" not in router["mainnet-v1"]


def test_node_exposes_the_same_mainnet_v1_surface() -> None:
    node = load_features("node/Cargo.toml")

    assert node["mainnet-v1"] == ["mainnet-rc1", "x3-chain-runtime/mainnet-v1"]


def test_router_scope_guard_permits_only_the_explicit_mainnet_v1_exception() -> None:
    source = (ROOT / "pallets/x3-cross-vm-router/src/lib.rs").read_text(encoding="utf-8")

    assert re.search(
        r'#\[cfg\(\s*all\(\s*feature = "mainnet-rc1",\s*'
        r'feature = "external-gateway",\s*'
        r'not\(feature = "mainnet-v1"\)\s*\)\s*\)\]',
        source,
    )

