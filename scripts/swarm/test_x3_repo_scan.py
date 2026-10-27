#!/usr/bin/env python3
"""Fixture-driven tests for the X3 repo scanner.

The scanner's job is to say true things about a tree, so the tests build trees:
one with nothing wrong in it (the negative control — a scanner that always finds
something fails here) and one with exactly one instance of each defect it claims
to detect. Assertions name the file and symbol, not just a count, because a
count-only assertion passes when the finding moves somewhere useless.

`swarm_scan_generates_report` is the name `FEATURE_REGISTRY.toml` cites for
`[repo_scanner_agent]`; `test_swarm_scan_generates_report` is the pytest-visible
alias, so the citation resolves to a function a gate actually runs.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
SCANNER_DIR = REPO / "scripts" / "swarm"
sys.path.insert(0, str(SCANNER_DIR))

import x3_repo_scan as scan_mod  # noqa: E402  (path setup above)

FINDING_FIELDS = (
    "id",
    "kind",
    "severity",
    "path",
    "line",
    "symbol",
    "why",
    "suggested_fix",
    "test_required",
    "gate_affected",
)

GATE_LIST = "\n".join(
    [
        "GATES_FAST=(",
        '  "format check:cargo fmt --all -- --check"',
        '  "test alpha:cargo test -p alpha-crate"',
        '  "expiry:bash scripts/wrap.sh"',
        ")",
        "",
    ]
)


def write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def clean_fixture(root: Path) -> None:
    """A tree with nothing for the scanner to report."""
    write(
        root / "FEATURE_REGISTRY.toml",
        "\n".join(
            [
                "[good_feature]",
                'crate_or_service = "pallets/good-pallet"',
                'required_tests = ["good_pallet_works"]',
                'proof_report = "docs/proofs/good.md"',
                "readiness_score = 80",
                "",
            ]
        ),
    )
    write(root / "docs/proofs/good.md", "proof\n")
    write(
        root / "pallets/good-pallet/Cargo.toml",
        '[package]\nname = "good-pallet"\nversion = "0.1.0"\n',
    )
    write(
        root / "pallets/good-pallet/src/lib.rs",
        "\n".join(
            [
                "pub trait WeightInfo {}",
                "#[pallet::call]",
                "impl<T: Config> Pallet<T> {}",
                "#[cfg(test)]",
                "mod tests {",
                "    #[test]",
                "    fn good_pallet_works() {}",
                "}",
                "",
            ]
        ),
    )
    write(root / "runtime/src/lib.rs", "GoodPallet: pallets::good_pallet,\n")
    write(
        root / "scripts/local-ci.sh",
        GATE_LIST.replace(
            '  "expiry:bash scripts/wrap.sh"',
            '  "test good:cargo test -p good-pallet"\n  "expiry:bash scripts/wrap.sh"',
        ),
    )


def dirty_fixture(root: Path) -> None:
    """One instance of each defect the scanner claims to detect."""
    write(
        root / "FEATURE_REGISTRY.toml",
        "\n".join(
            [
                "[ghost_test]",
                'crate_or_service = "scripts/swarm/swarm_scan.sh"',
                'required_tests = ["a_test_nobody_wrote"]',
                'proof_report = "docs/proofs/missing.md"',
                "readiness_score = 25",
                "",
                "[ghost_path]",
                'crate_or_service = "pallets/not-here"',
                "required_tests = []",
                "",
            ]
        ),
    )
    # A registered pallet charging an invented literal weight.
    write(
        root / "pallets/loud-pallet/Cargo.toml",
        '[package]\nname = "loud-pallet"\nversion = "0.1.0"\n',
    )
    write(
        root / "pallets/loud-pallet/src/lib.rs",
        "\n".join(
            [
                "#[pallet::call]",
                "impl<T: Config> Pallet<T> {",
                "    #[pallet::weight(Weight::from_parts(10_000, 0))]",
                "    pub fn poke(origin: OriginFor<T>) -> DispatchResult { Ok(()) }",
                "}",
                "",
            ]
        ),
    )
    # A pallet the runtime never names.
    write(
        root / "pallets/orphan-pallet/Cargo.toml",
        '[package]\nname = "orphan-pallet"\nversion = "0.1.0"\n',
    )
    write(
        root / "pallets/orphan-pallet/src/lib.rs",
        "#[pallet::call]\nimpl<T: Config> Pallet<T> {}\n",
    )
    write(root / "runtime/src/lib.rs", "LoudPallet: pallets::loud_pallet,\n")
    # The row above names this script; it exists, so the only missing path in the
    # registry is the one this fixture means to test.
    write(root / "scripts/swarm/swarm_scan.sh", "#!/usr/bin/env bash\n")
    # alpha-crate is gated by name; wrapped-crate only through scripts/wrap.sh,
    # which a gate invokes; lonely-crate is gated by nothing.
    for name in ("alpha-crate", "wrapped-crate", "lonely-crate"):
        write(
            root / f"crates/{name}/Cargo.toml",
            f'[package]\nname = "{name}"\nversion = "0.1.0"\n',
        )
        write(
            root / f"crates/{name}/src/lib.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_works() {}\n}\n",
        )
    write(root / "scripts/local-ci.sh", GATE_LIST)
    write(root / "scripts/wrap.sh", "#!/usr/bin/env bash\ncargo test -p wrapped-crate\n")


@pytest.fixture()
def dirty(tmp_path: Path) -> Path:
    dirty_fixture(tmp_path)
    return tmp_path


def scan_findings(root: Path) -> list[dict]:
    findings, summary = scan_mod.scan(root, with_gate_summary=False)
    payload = scan_mod.write_reports(root, findings, summary)
    return payload["findings"]


def by_kind(findings: list[dict], kind: str) -> list[dict]:
    return [f for f in findings if f["kind"] == kind]


def swarm_scan_generates_report() -> None:
    """The check `[repo_scanner_agent]` cites: the scanner finds the defects."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        dirty_fixture(root)
        findings = scan_findings(root)

        # Every finding is complete: a schema with holes is a report nobody can act on.
        assert findings, "the scanner found nothing in a tree built to contain defects"
        for finding in findings:
            for field in FINDING_FIELDS:
                assert field in finding, f"finding is missing {field}: {finding}"
            assert finding["severity"] in {"critical", "high", "medium", "low"}
            assert finding["why"].strip() and finding["suggested_fix"].strip()
            assert finding["test_required"].strip() and finding["gate_affected"].strip()

        stale = by_kind(findings, "stale-registry-test")
        assert [f["symbol"] for f in stale] == ["ghost_test:a_test_nobody_wrote"]
        ghost_paths = by_kind(findings, "missing-registry-path")
        assert [f["symbol"] for f in ghost_paths] == ["ghost_path: pallets/not-here"]
        assert by_kind(findings, "stale-proof-report"), "a missing proof_report must be reported"

        weights = by_kind(findings, "pallet-call-without-weights")
        assert [f["path"] for f in weights] == ["pallets/loud-pallet/src/lib.rs"]
        assert weights[0]["line"] > 0, "a weight finding must name the line"

        orphans = by_kind(findings, "unregistered-pallet")
        assert [f["symbol"] for f in orphans] == ["orphan-pallet"]

        ungated = sorted(f["symbol"] for f in by_kind(findings, "ungated-crate"))
        assert ungated == ["lonely-crate"], (
            "a crate gated by name (alpha-crate) or through a gate-invoked wrapper "
            f"(wrapped-crate) must not be reported; got {ungated}"
        )

        # Deterministic order: same tree, same findings, run after run.
        assert scan_findings(root) == findings

        # The Markdown report is written and carries the schema for each finding.
        report = root / scan_mod.REPORT_MD
        assert report.is_file(), "the scan must write its Markdown report"
        body = report.read_text()
        for finding in findings:
            assert finding["id"] in body
            assert finding["gate_affected"] in body


def test_swarm_scan_generates_report() -> None:
    swarm_scan_generates_report()


def test_clean_tree_produces_no_findings(tmp_path: Path) -> None:
    """Negative control: the scanner is not asserting a constant."""
    clean_fixture(tmp_path)
    assert scan_findings(tmp_path) == []


def test_patch_context_matches_the_file_it_patches(tmp_path: Path) -> None:
    """A patch we hand over must actually apply to the file it names."""
    dirty_fixture(tmp_path)
    ctx = scan_mod.ScanContext(tmp_path)
    findings, _ = scan_mod.scan(tmp_path, with_gate_summary=False)
    written = scan_mod.write_patches(ctx, findings)
    assert written, "an ungated crate must produce a patch"
    lines = (tmp_path / written[0]).read_text().splitlines()
    assert lines[0] == "--- a/scripts/local-ci.sh"
    assert lines[1] == "+++ b/scripts/local-ci.sh"
    header = lines[2]
    assert header.startswith("@@ -")
    start = int(header.split(" ")[1][1:].split(",")[0])
    context = [l[1:] for l in lines[3:] if l.startswith(" ")]
    target = (tmp_path / "scripts/local-ci.sh").read_text().splitlines()
    window = target[start - 1 : start - 1 + len(context)]
    assert window == context, (
        "the patch's context lines must match the file at the stated offset, or the "
        f"diff does not apply: {context} vs {window}"
    )
    assert any(l.startswith("+") and "lonely-crate" in l for l in lines[3:])


def test_a_shell_case_resolves_wherever_it_is_defined(tmp_path: Path) -> None:
    """`path::case` is the citation form this scanner asks script targets to use.

    The shell arm of `defined_symbols` anchored on `^` without `re.MULTILINE`, so
    `^` meant the start of the *file* and the only shell function it could ever
    see was one defined on line 0. Every real gate defines its cases under a
    shebang, so the citation form the scanner itself documents was unverifiable:
    a gate that ran, and passed, was still reported as citing a test that resolves
    nowhere. This is the same fixture read twice — the declaration moves, the
    finding has to move with it.
    """
    clean_fixture(tmp_path)
    write(
        tmp_path / "scripts/swarm/swarm_scan.sh",
        "#!/usr/bin/env bash\nset -euo pipefail\n\nswarm_scan_generates_report() {\n  echo scanned\n}\n",
    )
    write(
        tmp_path / "FEATURE_REGISTRY.toml",
        "\n".join(
            [
                "[shell_gate]",
                'crate_or_service = "scripts/swarm/swarm_scan.sh"',
                'required_tests = ["scripts/swarm/swarm_scan.sh::swarm_scan_generates_report"]',
                "readiness_score = 50",
                "",
            ]
        ),
    )
    assert "stale-registry-test" not in [f["kind"] for f in scan_findings(tmp_path)]

    # Negative control: the same shape under a different name is not a resolution
    # for this citation, so the finding has to come back.
    write(
        tmp_path / "scripts/swarm/swarm_scan.sh",
        "#!/usr/bin/env bash\nset -euo pipefail\n\na_different_case() {\n  echo scanned\n}\n",
    )
    stale = by_kind(scan_findings(tmp_path), "stale-registry-test")
    assert [f["symbol"] for f in stale] == ["shell_gate:swarm_scan_generates_report"], stale


def unwired_fixture(root: Path, runtime_names_it: bool = False) -> None:
    """A tree whose only `pallets/` crate is on the scanner's documented-unwired list."""
    write(
        root / "FEATURE_REGISTRY.toml",
        "\n".join(
            [
                "[x3_control_pallet]",
                'crate_or_service = "pallets/pallet-x3-control"',
                "required_tests = []",
                "readiness_score = 46",
                "",
            ]
        ),
    )
    write(
        root / "pallets/pallet-x3-control/Cargo.toml",
        '[package]\nname = "pallet-x3-control"\nversion = "0.1.0"\n',
    )
    write(root / "pallets/pallet-x3-control/src/lib.rs", "pub struct ControlState;\n")
    write(root / "feature-matrix/agents-experimental.toml", 'paths = ["pallets/pallet-x3-control"]\n')
    write(
        root / "runtime/src/lib.rs",
        "PalletX3Control: pallet_x3_control,\n" if runtime_names_it else "SomeOtherPallet: x,\n",
    )


def test_a_documented_unwired_pallet_is_a_decision_not_a_finding(tmp_path: Path) -> None:
    """An absence the registry records on purpose is rendered, not reported as a defect.

    `pallets/pallet-x3-control` is fail-closed and tested but no chain reads its state, and the
    owning feature row records that as a decision. The scanner has to be able to tell that apart
    from a pallet somebody forgot to wire — while still failing the moment the decision goes stale.
    """
    unwired_fixture(tmp_path)
    assert by_kind(scan_findings(tmp_path), "unregistered-pallet") == []

    documented, stale = scan_mod.documented_unwired(scan_mod.ScanContext(tmp_path))
    assert stale == []
    assert [row["package"] for row in documented] == ["pallet-x3-control"]
    assert documented[0]["owner"] == "feature-matrix/agents-experimental.toml"
    assert documented[0]["reason"], "a documented decision must carry its reason"


def test_a_documented_decision_goes_stale_when_the_runtime_names_the_pallet(tmp_path: Path) -> None:
    unwired_fixture(tmp_path, runtime_names_it=True)
    findings = by_kind(scan_findings(tmp_path), "unregistered-pallet")
    assert [f["symbol"] for f in findings] == ["stale-unwired-decision:pallet-x3-control"], findings
    assert "KNOWN_UNWIRED_PALLETS" in findings[0]["suggested_fix"]
    documented, stale = scan_mod.documented_unwired(scan_mod.ScanContext(tmp_path))
    assert documented == []
    assert len(stale) == 1 and "now names it" in stale[0]


def test_a_documented_decision_without_its_owner_document_is_reported(tmp_path: Path) -> None:
    unwired_fixture(tmp_path)
    (tmp_path / "feature-matrix/agents-experimental.toml").unlink()
    documented, stale = scan_mod.documented_unwired(scan_mod.ScanContext(tmp_path))
    assert documented == []
    assert len(stale) == 1 and "owning document" in stale[0]
    findings = by_kind(scan_findings(tmp_path), "unregistered-pallet")
    assert [f["symbol"] for f in findings] == ["stale-unwired-decision:pallet-x3-control"], findings


def test_an_unregistered_pallet_that_is_not_on_the_list_is_still_a_finding(tmp_path: Path) -> None:
    """The control: the list is keyed by package, so a second unwired pallet is reported."""
    unwired_fixture(tmp_path)
    write(
        tmp_path / "pallets/forgotten-pallet/Cargo.toml",
        '[package]\nname = "forgotten-pallet"\nversion = "0.1.0"\n',
    )
    write(tmp_path / "pallets/forgotten-pallet/src/lib.rs", "pub struct Nothing;\n")
    findings = by_kind(scan_findings(tmp_path), "unregistered-pallet")
    assert [f["symbol"] for f in findings] == ["forgotten-pallet"], findings


def weight_exception_fixture(root: Path, *, measured: bool = False, other_literal: bool = False) -> None:
    """A tree with one pallet whose named call is on the unmeasurable-weight list."""
    write(
        root / "pallets/x3-cross-vm-router/Cargo.toml",
        '[package]\nname = "pallet-x3-cross-vm-router"\nversion = "0.1.0"\n',
    )
    # The fixture reproduces the pallet's real shape: every call on the exception list is present, so
    # an entry only reads as stale when the tree genuinely stopped charging a literal for it.
    recorded = [
        "register_external_root",
        "xvm_transfer",
        "xvm_transfer_from_vm",
        "complete_xvm_transfer",
        "cancel_expired_xvm_transfer",
    ]
    calls = []
    for index, name in enumerate(recorded):
        if name == "register_external_root" and measured:
            weight = "#[pallet::weight(T::WeightInfo::register_external_root())]"
        else:
            weight = "#[pallet::weight(frame_support::weights::Weight::from_parts(50_000, 0))]"
        calls.append(
            f"#[pallet::call_index({index})]\n{weight}\n"
            f"pub fn {name}() -> DispatchResult {{ Ok(()) }}\n"
        )
    if other_literal:
        calls.append(
            "#[pallet::call_index(9)]\n"
            "#[pallet::weight(frame_support::weights::Weight::from_parts(40_000, 0))]\n"
            "pub fn unrecorded_call() -> DispatchResult { Ok(()) }\n"
        )
    write(
        root / "pallets/x3-cross-vm-router/src/lib.rs",
        "#[pallet::call]\nimpl Pallet {\n" + "".join(calls) + "}\n",
    )
    write(root / "runtime/src/lib.rs", "pallet_x3_cross_vm_router,\n")
    write(root / "docs/reports/benchmark-exceptions.md", "# recorded exceptions\n")


def test_a_recorded_unmeasurable_weight_is_a_decision_not_a_finding(tmp_path: Path) -> None:
    """A call no benchmark can reach is rendered with its reason, not reported as a defect.

    `register_external_root` reaches a production verifier backed by runtime-specific finalized
    header state, so the generic pallet benchmark cannot reach success without bypassing it. The
    scanner has to tell that apart from a literal somebody simply never measured — while still
    reporting every *other* literal in the same pallet, and while failing the moment the recorded
    call becomes measurable.
    """
    weight_exception_fixture(tmp_path)
    assert by_kind(scan_findings(tmp_path), "pallet-call-without-weights") == []

    documented, stale = scan_mod.documented_unmeasurable_calls(scan_mod.ScanContext(tmp_path))
    assert stale == []
    assert documented == {
        ("x3-cross-vm-router", "register_external_root"),
        ("x3-cross-vm-router", "xvm_transfer"),
        ("x3-cross-vm-router", "xvm_transfer_from_vm"),
        ("x3-cross-vm-router", "complete_xvm_transfer"),
        ("x3-cross-vm-router", "cancel_expired_xvm_transfer"),
    }


def test_a_recorded_weight_exception_goes_stale_when_the_call_is_measured(tmp_path: Path) -> None:
    weight_exception_fixture(tmp_path, measured=True)
    documented, stale = scan_mod.documented_unmeasurable_calls(scan_mod.ScanContext(tmp_path))
    # Only the call that is measured now goes stale; the others are still live decisions.
    assert ("x3-cross-vm-router", "register_external_root") not in documented
    assert len(documented) == 4
    assert len(stale) == 1 and "measured now" in stale[0]
    findings = by_kind(scan_findings(tmp_path), "pallet-call-without-weights")
    assert [f["symbol"] for f in findings] == [
        "stale-weight-exception:pallet-x3-cross-vm-router::register_external_root"
    ], findings
    assert "UNMEASURABLE_CALL_WEIGHTS" in findings[0]["suggested_fix"]


def test_a_literal_outside_the_recorded_exceptions_is_still_a_finding(tmp_path: Path) -> None:
    """The control: the list is keyed per call, so a second literal in the same pallet is reported."""
    weight_exception_fixture(tmp_path, other_literal=True)
    findings = by_kind(scan_findings(tmp_path), "pallet-call-without-weights")
    assert [f["symbol"] for f in findings] == [
        "x3-cross-vm-router::frame_support::weights::Weight::from_parts(40_000, 0"
    ], findings


def test_scanner_is_the_registry_citation() -> None:
    """Every test `[repo_scanner_agent]` cites must be a function this module defines.

    The readiness gate cannot check this row (its `crate_or_service` is a script,
    not a directory of `.rs` files), so the row checks itself — which is the whole
    reason the scanner exists.
    """
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover - Python 3.10
        import tomli as tomllib

    registry = tomllib.loads((REPO / "FEATURE_REGISTRY.toml").read_text())
    row = registry["repo_scanner_agent"]
    assert "swarm_scan_generates_report" in row["required_tests"]
    for name in row["required_tests"]:
        assert callable(globals().get(name)), (
            f"[repo_scanner_agent] cites `{name}`, which this module does not define"
        )
    assert (REPO / row["proof_report"]).is_file(), "the row must cite a report that exists"
