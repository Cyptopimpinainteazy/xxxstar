#!/usr/bin/env python3
"""Is every external (cross-chain) path still refusing to run?

The public-testnet goal states the rule for this area outright: an EVM/SVM
*external* path is either proven against a real testnet or it is explicitly
disabled. Nothing in this workspace has verified a settlement proof against a
public network, so every external path has to be closed in the default
configuration — and something has to keep it that way.

Until now nothing did. The refusals were real and mostly tested, but they were
spread across four crates, no single check named them together, and
`x3-external-chains` — the crate that holds every EVM adapter *and* the external
settlement verifier — was in no `scripts/local-ci.sh` gate list at all, so its
refusal suites ran nowhere. This check is that missing single command.

The external paths, and where each one is gated today:

  1. x3-cross-vm-router — external bridge root registration and the external
     route surface. `ExternalBridgesEnabled` is a `ValueQuery` storage value
     (false, and there is no genesis field that could seed it true);
     `register_external_root` refuses with `ExternalBridgesDisabled`; the only
     way to open it is `set_external_bridges_enabled`, which is root-only and
     additionally requires `ExternalBridgeAuditGate` to have been set.
  2. x3-settlement-engine — a cross-domain proof set that summarises an
     *external* leg. `AllowUnattestedCrossDomainProofs` is a `ValueQuery`
     storage value (false by default, and false at genesis for every chain a
     validator can join); `require_verified_external_bundle` refuses with
     `CrossDomainProofUnverified` unless the bundle names a transaction the
     pallet's own verifier recorded.
  3. x3-verification-router — the proof-verification dispatch. A strategy with
     no verifier answers `NotImplemented`, an `Unsupported` strategy answers
     `InvalidStrategy`, and the permissive `TestOnly` verifier cannot compile in
     a `production` build (`compile_error!`).
  4. x3-external-chains — every EVM adapter and the settlement verifier. Adapter
     sends answer `AdapterUnimplemented`, adapter proof checks answer
     `VerificationUnavailable`, and each of the five `SettlementVerifier`
     `verify_*` bodies answers `VerificationUnavailable`. None answers `Ok(true)`.
  5. BTC external gateway — `btc_mainnet_gateway` names the exposure and the
     registry records the path as simulator-only.

Each path is checked three ways, so a static claim cannot drift away from a
behavioural proof:

  * the code gate that refuses it exists and is the *closed* value,
  * the test that shows the refusal is present *and* its gate is wired into
    `scripts/local-ci.sh` (an untested refusal is not evidence), and
  * where a flag names the exposure, the flag in `TESTNET_FEATURE_FLAGS.toml`
    is `DISABLED_BLOCKED`.

`node/src/chain_spec.rs` is parsed as well: a chain a validator can join
(`ChainType::Live`) must be born with `allow_unattested_cross_domain_proofs =
false`, and only a dev/local spec may pass `true`. A live spec that passes a
non-literal is a violation, because "we could not tell" is not "it is closed".

Load-bearing, measured 2026-09-26: flipping `external_bridges_mainnet` to
`GUARDED_TESTNET`, and flipping the staging spec's `allow_unattested` argument
to `true`, each make this exit 1 with the offending line named (see the
workstream report). `--local-ci PATH` exists only so that reversal can be shown
against a scratch copy of the gate list without editing the primary agent's
file; CI runs this with no argument.

Usage:
    scripts/ci/check-external-paths-disabled.py            # check
    scripts/ci/check-external-paths-disabled.py --list     # print the path table
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

DEFAULT_LOCAL_CI = ROOT / "scripts/local-ci.sh"


# ── model ────────────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class CodeCheck:
    """A required shape in a source file."""

    path: str
    pattern: str
    message: str


@dataclass(frozen=True)
class Proof:
    """A test that shows the refusal happens, and the gate that runs it."""

    path: str
    test: str
    gate: str


@dataclass(frozen=True)
class Flag:
    """A `TESTNET_FEATURE_FLAGS.toml` key that names the exposure."""

    key: str
    expected: str


@dataclass(frozen=True)
class ExternalPath:
    name: str
    gate: str
    refusal: str
    checks: list[CodeCheck] = field(default_factory=list)
    proofs: list[Proof] = field(default_factory=list)
    flags: list[Flag] = field(default_factory=list)


PATHS: list[ExternalPath] = [
    ExternalPath(
        name="x3-cross-vm-router: external bridge surface",
        gate="pallets/x3-cross-vm-router/src/lib.rs — ExternalBridgesEnabled storage gate",
        refusal="Error::ExternalBridgesDisabled on register_external_root; "
        "root-only enable requires ExternalBridgeAuditGate",
        checks=[
            CodeCheck(
                "pallets/x3-cross-vm-router/src/lib.rs",
                r"ExternalBridgesEnabled<T: Config>\s*=\s*StorageValue<_,\s*bool,\s*ValueQuery>",
                "ExternalBridgesEnabled must be a ValueQuery storage value, so it is false at "
                "genesis and no genesis field can seed it true",
            ),
            CodeCheck(
                "pallets/x3-cross-vm-router/src/lib.rs",
                r"ExternalBridgeAuditGate<T: Config>\s*=\s*StorageValue<_,\s*bool,\s*ValueQuery>",
                "ExternalBridgeAuditGate must be a ValueQuery storage value (false until governance "
                "records an audit)",
            ),
            CodeCheck(
                "pallets/x3-cross-vm-router/src/lib.rs",
                r"ensure!\(\s*ExternalBridgesEnabled::<T>::get\(\),\s*Error::<T>::ExternalBridgesDisabled",
                "register_external_root must refuse while external bridges are disabled",
            ),
            CodeCheck(
                "pallets/x3-cross-vm-router/src/lib.rs",
                r"pub fn set_external_bridges_enabled\([\s\S]{0,200}?ensure_root\(origin\)\?",
                "set_external_bridges_enabled must be root-only (ensure_root) so no signed origin "
                "can open the external surface",
            ),
            CodeCheck(
                "pallets/x3-cross-vm-router/src/lib.rs",
                r"ExternalBridgeAuditGate::<T>::get\(\),\s*Error::<T>::ExternalBridgeAuditGateMissing",
                "enabling external bridges must require the documented audit gate",
            ),
        ],
        proofs=[
            Proof(
                "pallets/x3-cross-vm-router/src/tests.rs",
                "external_bridges_are_paused_at_genesis",
                "test x3-cross-vm-router",
            ),
            Proof(
                "pallets/x3-cross-vm-router/src/tests.rs",
                "enabling_external_bridges_requires_documented_audit_gate",
                "test x3-cross-vm-router",
            ),
            Proof(
                "pallets/x3-cross-vm-router/src/tests.rs",
                "revoking_bridge_audit_gate_disables_external_bridges",
                "test x3-cross-vm-router",
            ),
        ],
        flags=[Flag("external_bridges_mainnet", "DISABLED_BLOCKED")],
    ),
    ExternalPath(
        name="x3-settlement-engine: unattested external proof sets",
        gate="pallets/x3-settlement-engine/src/lib.rs — AllowUnattestedCrossDomainProofs + "
        "require_verified_external_bundle",
        refusal="Error::CrossDomainProofUnverified when no verified proof backs the external leg",
        checks=[
            CodeCheck(
                "pallets/x3-settlement-engine/src/lib.rs",
                r"AllowUnattestedCrossDomainProofs<T: Config>\s*=\s*StorageValue<_,\s*bool,\s*ValueQuery>",
                "AllowUnattestedCrossDomainProofs must be a ValueQuery storage value (false unless "
                "genesis explicitly passes true)",
            ),
            CodeCheck(
                "pallets/x3-settlement-engine/src/lib.rs",
                r"fn require_verified_external_bundle\(",
                "the verified-external-bundle gate must exist",
            ),
            CodeCheck(
                "pallets/x3-settlement-engine/src/lib.rs",
                r"Error::<T>::CrossDomainProofUnverified",
                "the gate must refuse with CrossDomainProofUnverified",
            ),
        ],
        proofs=[
            Proof(
                "pallets/x3-settlement-engine/src/tests.rs",
                "live_posture_refuses_an_external_bundle_no_verifier_backed",
                "test settlement-engine",
            ),
            # Positive control: the same gate accepts a bundle that does name a
            # verified proof, so it is not a constant "always refuse".
            Proof(
                "pallets/x3-settlement-engine/src/tests.rs",
                "a_bundle_matching_the_verified_proof_is_accepted",
                "test settlement-engine",
            ),
            # ...and the rule itself flips with the `allow_unattested` posture,
            # which is what makes the gate load-bearing rather than a constant.
            Proof(
                "pallets/x3-settlement-engine/src/tests.rs",
                "the_rule_for_requiring_a_verified_proof_is_explicit",
                "test settlement-engine",
            ),
        ],
    ),
    ExternalPath(
        name="x3-verification-router: external proof dispatch",
        gate="crates/x3-verification-router/src/lib.rs — strategy dispatch + production/test-verifier "
        "compile_error!",
        refusal="VerificationError::NotImplemented for a strategy with no verifier; "
        "InvalidStrategy for Unsupported",
        checks=[
            CodeCheck(
                "crates/x3-verification-router/src/lib.rs",
                r'#\[cfg\(all\(feature = "production", feature = "test-verifier"\)\)\]\s*compile_error!',
                "the permissive test verifier must be a compile error in a production build",
            ),
            CodeCheck(
                "crates/x3-verification-router/src/lib.rs",
                r"VerificationStrategy::Unsupported => return Err\(VerificationError::InvalidStrategy\)",
                "an Unsupported strategy must fail closed with InvalidStrategy",
            ),
            CodeCheck(
                "crates/x3-verification-router/src/lib.rs",
                r"Err\(VerificationError::NotImplemented\)",
                "a strategy with no registered verifier must fail closed with NotImplemented",
            ),
        ],
        proofs=[
            Proof(
                "crates/x3-verification-router/src/lib.rs",
                "unimplemented_strategies_fail_closed",
                "test verification router",
            ),
            Proof(
                "crates/x3-verification-router/src/lib.rs",
                "missing_verifier_fails",
                "test verification router",
            ),
            Proof(
                "crates/x3-verification-router/src/lib.rs",
                "unsupported_strategy_fails",
                "test verification router",
            ),
            # Flipping the `test-verifier` feature *is* what changes the answer,
            # so the production refusal is a feature gate and not a constant.
            Proof(
                "crates/x3-verification-router/src/lib.rs",
                "evm_receipt_verifier_works_under_test_verifier",
                "test verification router test-verifier",
            ),
        ],
    ),
    ExternalPath(
        name="x3-external-chains: EVM adapters + settlement verifier",
        gate="crates/external-chains/src/settlement.rs — every verify_* body; "
        "crates/external-chains/src/error.rs — the refusal variants",
        refusal="ExternalChainError::AdapterUnimplemented for sends/status/finalize; "
        "VerificationUnavailable for every proof check",
        checks=[
            CodeCheck(
                "crates/external-chains/src/error.rs",
                r"AdapterUnimplemented\(Vec<u8>\)",
                "the AdapterUnimplemented refusal variant must exist",
            ),
            CodeCheck(
                "crates/external-chains/src/error.rs",
                r"\bVerificationUnavailable\b",
                "the VerificationUnavailable refusal variant must exist",
            ),
            CodeCheck(
                "crates/external-chains/src/settlement.rs",
                r"Err\(ExternalChainError::VerificationUnavailable\)",
                "SettlementVerifier must refuse with VerificationUnavailable",
            ),
        ],
        proofs=[
            Proof(
                "crates/external-chains/src/settlement.rs",
                "every_proof_type_is_refused_while_unimplemented",
                "test x3-external-chains",
            ),
            Proof(
                "crates/external-chains/tests/adapters_refuse_unimplemented_operations.rs",
                "no_adapter_accepts_a_proof_it_cannot_verify",
                "test x3-external-chains",
            ),
            Proof(
                "crates/external-chains/tests/external_proof_paths_fail_closed.rs",
                "every_chain_refuses_every_proof_type_it_has_no_verifier_for",
                "test x3-external-chains",
            ),
        ],
    ),
    ExternalPath(
        name="BTC external gateway",
        gate="TESTNET_FEATURE_FLAGS.toml (exposure) + FEATURE_REGISTRY.toml btc_fortress_gateway "
        "(simulator-only)",
        refusal="BTC mainnet path OFF; simulator-only until a signer quorum and audit exist",
        checks=[],
        proofs=[],
        flags=[Flag("btc_mainnet_gateway", "DISABLED_BLOCKED")],
    ),
]

#: The `SettlementVerifier` bodies that must refuse, by name.
VERIFY_BODIES = (
    "verify_merkle_proof",
    "verify_light_client_proof",
    "verify_zk_proof",
    "verify_signature_proof",
    "verify_optimistic_proof",
)


# ── helpers ──────────────────────────────────────────────────────────────────


def read(rel: str) -> str:
    path = ROOT / rel
    if not path.is_file():
        raise SystemExit(f"check-external-paths-disabled: missing file {rel}")
    return path.read_text(encoding="utf-8", errors="replace")


def find_flags(text: str) -> dict[str, str]:
    """`key = "VALUE"` assignments, the shape TESTNET_FEATURE_FLAGS.toml uses."""
    return dict(re.findall(r'^\s*([a-z0-9_]+)\s*=\s*"([^"]*)"', text, re.MULTILINE))


def gate_slugs(local_ci: str) -> set[str]:
    """The names of the gates wired into the `scripts/local-ci.sh` arrays."""
    return set(re.findall(r'^\s*"([^":]+):', local_ci, re.MULTILINE))


def split_top_level(text: str) -> list[str]:
    """Split on commas that are not inside brackets or string literals."""
    parts: list[str] = []
    depth = 0
    in_string = False
    current: list[str] = []
    for char in text:
        if in_string:
            current.append(char)
            if char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
            current.append(char)
        elif char in "([{<":
            depth += 1
            current.append(char)
        elif char in ")]}>":
            depth -= 1
            current.append(char)
        elif char == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    tail = "".join(current).strip()
    if tail:
        parts.append(tail)
    return parts


def brace_body(source: str, open_brace: int) -> str:
    """The text between `open_brace` and its matching `}`."""
    depth = 0
    in_string = False
    for i in range(open_brace, len(source)):
        char = source[i]
        if in_string:
            if char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[open_brace + 1 : i]
    return ""


def strip_line_comments(text: str) -> str:
    """Drop `// …` tails so a comment cannot satisfy (or break) a code check."""
    return "\n".join(line.split("//", 1)[0] for line in text.splitlines())


def parse_chain_genesis_calls(source: str) -> list[tuple[str, list[str], str]]:
    """`(fn name, top-level arguments, chain type)` per `x3_chain_genesis(…)` call.

    The function *definition* is skipped: it is a parameter list, not a spec.
    """
    functions = list(re.finditer(r"pub fn ([a-z0-9_]+)\(", source))
    calls: list[tuple[str, list[str], str]] = []
    for match in re.finditer(r"(?<!fn )x3_chain_genesis\(", source):
        enclosing = [f for f in functions if f.start() < match.start()]
        fn_name = enclosing[-1].group(1) if enclosing else "<unknown>"
        body_end = next(
            (f.start() for f in functions if f.start() > match.start()), len(source)
        )
        body = source[match.start() : body_end]
        chain_type = re.search(r"with_chain_type\(ChainType::(\w+)\)", body)

        open_paren = source.index("(", match.start())
        depth = 0
        end = -1
        in_string = False
        for i in range(open_paren, len(source)):
            char = source[i]
            if in_string:
                if char == '"':
                    in_string = False
                continue
            if char == '"':
                in_string = True
            elif char in "([{<":
                depth += 1
            elif char in ")]}>":
                depth -= 1
                if depth == 0:
                    end = i
                    break
        if end < 0:
            raise SystemExit("check-external-paths-disabled: unbalanced x3_chain_genesis( call")

        args = split_top_level(source[open_paren + 1 : end])
        calls.append((fn_name, args, chain_type.group(1) if chain_type else "?"))
    return calls


# ── the checks ───────────────────────────────────────────────────────────────


def check_code(violations: list[str]) -> None:
    cache: dict[str, str] = {}
    for path in PATHS:
        for check in path.checks:
            text = cache.setdefault(check.path, read(check.path))
            if not re.search(check.pattern, text):
                violations.append(f"{path.name}: {check.message}  [{check.path}]")


def check_verifier_never_accepts(violations: list[str]) -> None:
    """Every `SettlementVerifier::verify_*` body must refuse, and never accept."""
    text = read("crates/external-chains/src/settlement.rs")

    for name in VERIFY_BODIES:
        match = re.search(rf"fn {name}\(", text)
        if not match:
            violations.append(
                "x3-external-chains: EVM adapters + settlement verifier: "
                f"`SettlementVerifier::{name}` is gone  [crates/external-chains/src/settlement.rs]"
            )
            continue

        body = strip_line_comments(brace_body(text, text.index("{", match.end())))
        if "VerificationUnavailable" not in body:
            violations.append(
                "x3-external-chains: EVM adapters + settlement verifier: "
                f"`SettlementVerifier::{name}` no longer refuses with VerificationUnavailable  "
                "[crates/external-chains/src/settlement.rs]"
            )
        if "Ok(true)" in body:
            violations.append(
                "x3-external-chains: EVM adapters + settlement verifier: "
                f"`SettlementVerifier::{name}` answers `Ok(true)` — an external proof this crate "
                "cannot verify must never be accepted  "
                "[crates/external-chains/src/settlement.rs]"
            )


def check_proofs(violations: list[str], slugs: set[str]) -> None:
    cache: dict[str, str] = {}
    for path in PATHS:
        for proof in path.proofs:
            text = cache.setdefault(proof.path, read(proof.path))
            if not re.search(rf"fn\s+{re.escape(proof.test)}\s*\(", text):
                violations.append(
                    f"{path.name}: the refusal proof `{proof.test}` is gone  [{proof.path}]"
                )
            if proof.gate not in slugs:
                violations.append(
                    f"{path.name}: the proof `{proof.test}` runs in no gate: add "
                    f'"{proof.gate}:cargo test …" to scripts/local-ci.sh or the refusal is untested'
                )


def check_flags(violations: list[str]) -> None:
    flags = find_flags(read("TESTNET_FEATURE_FLAGS.toml"))
    for path in PATHS:
        for flag in path.flags:
            actual = flags.get(flag.key)
            if actual is None:
                violations.append(
                    f"{path.name}: TESTNET_FEATURE_FLAGS.toml no longer names `{flag.key}`, so the "
                    "exposure flag a launch gate reads has been removed"
                )
            elif actual != flag.expected:
                violations.append(
                    f"{path.name}: `{flag.key} = \"{actual}\"` — an external path may only be "
                    f'"{flag.expected}" until it is proven against a real testnet'
                )


def check_chain_specs(violations: list[str]) -> None:
    """A chain a validator can join must be born refusing unattested proofs."""
    source = read("node/src/chain_spec.rs")

    live_seen = 0
    for fn_name, args, chain_type in parse_chain_genesis_calls(source):
        if len(args) < 8:
            violations.append(
                f"node/src/chain_spec.rs: {fn_name} calls x3_chain_genesis with {len(args)} "
                "arguments; the allow_unattested argument is at index 7"
            )
            continue
        value = args[7]
        if value not in ("true", "false"):
            violations.append(
                f"node/src/chain_spec.rs: {fn_name} passes `{value}` for "
                "allow_unattested_cross_domain_proofs; only a literal `false` proves the live "
                "posture is closed"
            )
            continue
        if chain_type == "Live":
            live_seen += 1
            if value != "false":
                violations.append(
                    f"node/src/chain_spec.rs: {fn_name} (ChainType::Live) is born with "
                    "allow_unattested_cross_domain_proofs = true — a chain a validator can join "
                    "must refuse unattested external proofs"
                )
        elif chain_type in ("Development", "Local") and value != "true":
            violations.append(
                f"node/src/chain_spec.rs: {fn_name} (ChainType::{chain_type}) is born with "
                "allow_unattested_cross_domain_proofs = false; the dev/local lifecycle tests "
                "depend on the bookkeeping posture being permissive"
            )

    if live_seen == 0:
        violations.append(
            "node/src/chain_spec.rs: no ChainType::Live chain spec was found, so the live posture "
            "was not checked at all"
        )


# ── entry point ──────────────────────────────────────────────────────────────


def print_table() -> None:
    for path in PATHS:
        print(f"* {path.name}")
        print(f"    gated by : {path.gate}")
        print(f"    reached  : {path.refusal}")
        for flag in path.flags:
            print(f"    flag     : {flag.key} = {flag.expected}")
        for proof in path.proofs:
            print(f"    proof    : {proof.test}  ({proof.gate})")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list", action="store_true", help="print the external path table")
    parser.add_argument(
        "--local-ci",
        default=str(DEFAULT_LOCAL_CI),
        help="gate list to read (default: scripts/local-ci.sh); exists for the reversal demo",
    )
    args = parser.parse_args()

    if args.list:
        print_table()
        return 0

    local_ci = Path(args.local_ci)
    if not local_ci.is_file():
        print(f"check-external-paths-disabled: no gate list at {local_ci}", file=sys.stderr)
        return 1

    violations: list[str] = []
    check_code(violations)
    check_verifier_never_accepts(violations)
    check_proofs(violations, gate_slugs(local_ci.read_text(encoding="utf-8", errors="replace")))
    check_flags(violations)
    check_chain_specs(violations)

    if violations:
        print(
            "check-external-paths-disabled: FAIL: an external path is not closed by default:",
            file=sys.stderr,
        )
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    checked = sum(len(p.checks) for p in PATHS) + len(PATHS) + sum(len(p.proofs) for p in PATHS)
    print(
        f"check-external-paths-disabled: OK — {len(PATHS)} external paths closed "
        f"({checked} gate shapes, proofs and flags checked; no settlement proof is accepted that "
        "this workspace cannot verify)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
