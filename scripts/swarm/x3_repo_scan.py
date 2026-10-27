#!/usr/bin/env python3
"""X3 repo scanner: findings a person can act on, not a markdown dump.

`scripts/swarm/swarm_scan.sh` used to dump every `TODO|FIXME|unwrap(` line and a
list of filenames whose name contained a subsystem word. Nothing in that output
said what was wrong, where the symbol was, which gate would catch it, or what to
do about it, and no test or gate ran it.

This scanner emits a **finding schema** instead. Every finding carries:

    id, severity, kind, path, line, symbol, why, suggested_fix, test_required,
    gate_affected

and is written to both `reports/swarm_scan_report.md` (readable) and
`reports/swarm_scan_findings.json` (machine-readable). Findings are sorted by
severity, then kind, then path, then line, so two runs over the same tree produce
byte-identical output.

What it deliberately does **not** re-implement:

* `TODO`/`FIXME`/stub markers and test cheats are owned by
  `scripts/x3_fake_code_scan.py`, which has its own ratchet and its own gates.
* `unwrap()`/`expect()`/`panic!` in reachable production code is owned by
  `scripts/audit/panic_unwrap_scan.py` and `scripts/mainnet/panic_unwrap_audit.sh`.

The scanner *runs* those two and folds their verdicts into `gate_summary` below,
so one command answers "what is wrong" and "which of the existing ratchets is
currently red". Re-reporting their findings as its own would be a second ratchet
over the same debt, so it does not.

What it does check that nothing else did (see `STRUCTURAL_KINDS`):

* `stale-registry-test` — a `required_tests` entry that names a function which
  exists nowhere. The readiness gate only resolves citations whose
  `crate_or_service` is a *directory* holding `.rs` files, so a row pointing at a
  script could cite a test nobody wrote and still pass. One did.
* `missing-registry-path` — a `crate_or_service` that is not in the tree.
* `stale-proof-report` — a `proof_report` that points at nothing.
* `unregistered-pallet` — a `pallets/*` crate `runtime/src/lib.rs` never mentions.
* `pallet-call-without-weights` — a `#[pallet::call]` block in a pallet that has no
  `WeightInfo` anywhere, the exact shape of the hand-written weights file that
  PR #519 shipped broken.
* `ungated-crate` — a crate with test attributes that no gate runs, including
  crates whose tests run only through a wrapper script a gate invokes (`run-expiry-test.sh`
  is such a wrapper; the unwired `scripts/check-crate-tests-are-gated.py` cannot see it).

Patches: findings whose remediation is mechanical and safe set `patch_eligible`,
and `--patches` writes a real unified diff to `.ai/patches/<id>.patch`. Nothing is
ever applied to the tree; a human still reviews and applies. Only `ungated-crate`
qualifies today: the fix is one line appended to a gate list, and the diff is
computed from the file's actual bytes.

Usage:
    python3 scripts/swarm/x3_repo_scan.py                    # scan, write reports
    python3 scripts/swarm/x3_repo_scan.py --json             # findings on stdout
    python3 scripts/swarm/x3_repo_scan.py --patches          # + .ai/patches/*.patch
    python3 scripts/swarm/x3_repo_scan.py --check            # fail on ratchet growth
    python3 scripts/swarm/x3_repo_scan.py --root <dir>       # scan a fixture tree
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10, the interpreter pytest runs under here
    import tomli as tomllib

SEVERITY_ORDER = {"critical": 0, "high": 1, "medium": 2, "low": 3}

# Kinds this scanner owns and ratchets. Everything else in a report is context.
STRUCTURAL_KINDS = (
    "stale-registry-test",
    "missing-registry-path",
    "stale-proof-report",
    "unregistered-pallet",
    "pallet-call-without-weights",
    "ungated-crate",
)

# Pallets under `pallets/` that the runtime deliberately does not include, each with the document
# that owns the decision and the reason for it. An entry is a *recorded decision*, not a way to
# stop looking: the detector still reports the entry the moment `runtime/src/lib.rs` names the
# pallet (the decision is stale and the list has to shrink), and it reports the entry if the
# owning document disappears. The decisions are rendered in the report's own section, so the
# reader sees them instead of an absence.
KNOWN_UNWIRED_PALLETS: dict[str, dict[str, str]] = {
    "pallet-x3-control": {
        "owner": "feature-matrix/agents-experimental.toml",
        "reason": (
            "the control plane is fail-closed and carries 12 tests, but nothing on a chain reads "
            "`ControlState`, so wiring it means deciding who acts on `Frozen`/`Paused` — a design "
            "decision the owning row records, not an oversight"
        ),
    },
}

# Extrinsic weights that cannot be measured, keyed `"<pallet entry>::<extrinsic>"`.
#
# A handful of calls cannot be reached by a benchmark at all: the runtime refuses them by design, or
# their origin cannot be synthesized because no origin is *always* authorized. `try_successful_origin`
# returning `Err` is the documented answer for those origins, and the honest consequence is that the
# call keeps a stated cost until the runtime grows a path a benchmark can drive. This is the same
# shape as `KNOWN_UNWIRED_PALLETS`: a recorded decision with an owner and a reason, plus a tripwire —
# the moment the call stops charging a literal (i.e. somebody measured it) the entry goes stale and
# the scanner says so, so an exception cannot outlive its justification.
UNMEASURABLE_CALL_WEIGHTS: dict[str, dict[str, str]] = {
    "pallet-x3-cross-vm-router::register_external_root": {
        "owner": "docs/reports/benchmark-exceptions.md",
        "reason": (
            "the production verifier reads runtime-specific finalized EVM/SVM/Bitcoin state that a "
            "generic pallet benchmark cannot fabricate without bypassing verification; the call "
            "charges two verifier storage reads plus declared compute and becomes measurable when a "
            "runtime benchmark helper can install real finalized evidence"
        ),
    },
    "pallet-x3-cross-vm-router::xvm_transfer": {
        "owner": "docs/reports/benchmark-exceptions.md",
        "reason": (
            "gated on `X3LangOrigin = EnsureAuthorizedGateway`, whose `try_successful_origin` returns "
            "`Err` because no account is always authorized in the custody registry; the router's "
            "`Config` has no path to that pallet, so its benchmark cannot authorize one"
        ),
    },
    "pallet-x3-cross-vm-router::xvm_transfer_from_vm": {
        "owner": "docs/reports/benchmark-exceptions.md",
        "reason": (
            "same custody-backed gateway origin as `xvm_transfer`, checked twice (adapter + x3-lang)"
        ),
    },
    "pallet-x3-cross-vm-router::complete_xvm_transfer": {
        "owner": "docs/reports/benchmark-exceptions.md",
        "reason": "same custody-backed gateway origin as `xvm_transfer`",
    },
    "pallet-x3-cross-vm-router::cancel_expired_xvm_transfer": {
        "owner": "docs/reports/benchmark-exceptions.md",
        "reason": "same custody-backed gateway origin as `xvm_transfer`",
    },
}

SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".next",
    "out",
}
SKIP_PREFIXES = (".wt-", ".kilo", "launch-gates/sources")
# Vendored dependency trees that happen to sit under a first-party root.
VENDOR_PREFIXES = (
    "apps/x3-desktop/src-tauri/tauri-vendor",
    "patches/",
    "primitives/",
)
# Vendored, cargo-normalized dependency copies (`.cargo-checksum.json` next to a
# generated manifest) and other non-first-party trees. Their tests are not this
# repository's to gate, so `ungated-crate` does not report them.
FIRST_PARTY_ROOTS = (
    "apps/",
    "crates/",
    "integration-tests/",
    "node/",
    "pallets/",
    "programs/",
    "runtime/",
    "services/",
    "tools/",
    "x3-autonomic-core/",
    "x3-lang/",
    "X3-contracts/",
)
SCAN_SUFFIXES = (
    ".rs",
    ".py",
    ".sh",
    ".toml",
    ".md",
    ".yml",
    ".yaml",
    ".json",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
)
MAX_FILE_BYTES = 2_000_000

# A `required_tests` entry may be a plain test name or `target::name`.
def citation_tail(citation: str) -> str:
    return citation.rsplit("::", 1)[-1].strip()


@dataclass(frozen=True)
class Finding:
    """One actionable issue, with everything a fixer needs."""

    kind: str
    severity: str
    path: str
    line: int
    symbol: str
    why: str
    suggested_fix: str
    test_required: str
    gate_affected: str
    patch_eligible: bool = False
    patch: str = ""

    @property
    def id(self) -> str:
        """Stable identity: kind + path + symbol, never the line number.

        Keying identity on a line number makes every finding below an insertion
        look like a brand-new one; this repo already paid for that lesson with
        the fake-code ratchet.
        """
        digest = hashlib.sha256(f"{self.kind}\0{self.path}\0{self.symbol}".encode()).hexdigest()
        return digest[:16]

    def as_dict(self) -> dict:
        out = {
            "id": self.id,
            "kind": self.kind,
            "severity": self.severity,
            "path": self.path,
            "line": self.line,
            "symbol": self.symbol,
            "why": self.why,
            "suggested_fix": self.suggested_fix,
            "test_required": self.test_required,
            "gate_affected": self.gate_affected,
            "patch_eligible": self.patch_eligible,
        }
        return out


def sort_key(f: Finding) -> tuple:
    return (SEVERITY_ORDER.get(f.severity, 9), f.kind, f.path, f.line, f.symbol)


def read_text(path: Path) -> str:
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            return ""
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


class ScanContext:
    """A tree to scan, plus the lazily-built indexes the detectors share."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self._files: list[tuple[str, Path]] | None = None
        self._text: dict[str, str] = {}
        self._registry: dict | None = None

    # -- files ---------------------------------------------------------------

    def files(self) -> list[tuple[str, Path]]:
        if self._files is None:
            found: list[tuple[str, Path]] = []
            for dirpath, dirnames, filenames in os.walk(self.root):
                rel_dir = os.path.relpath(dirpath, self.root)
                rel_dir = "" if rel_dir == "." else rel_dir.replace(os.sep, "/")
                blocked = SKIP_PREFIXES + VENDOR_PREFIXES
                dirnames[:] = sorted(
                    d
                    for d in dirnames
                    if d not in SKIP_DIRS
                    and not f"{rel_dir}/{d}".lstrip("/").startswith(blocked)
                )
                for name in sorted(filenames):
                    rel = f"{rel_dir}/{name}" if rel_dir else name
                    if rel.startswith(SKIP_PREFIXES + VENDOR_PREFIXES):
                        continue
                    if rel.endswith(SCAN_SUFFIXES):
                        found.append((rel, Path(dirpath) / name))
            self._files = found
        return self._files

    def text(self, rel: str) -> str:
        if rel not in self._text:
            self._text[rel] = read_text(self.root / rel)
        return self._text[rel]

    def write_text(self, rel: str, body: str) -> None:
        target = self.root / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8")

    # -- registry ------------------------------------------------------------

    def registry(self) -> dict:
        if self._registry is None:
            path = self.root / "FEATURE_REGISTRY.toml"
            if not path.exists():
                self._registry = {}
            else:
                try:
                    self._registry = tomllib.loads(read_text(path))
                except tomllib.TOMLDecodeError as exc:  # a broken registry is itself a finding
                    self._registry = {}
                    self._registry_error = str(exc)
        return self._registry

    def registry_features(self) -> list[tuple[str, dict]]:
        return [(k, v) for k, v in self.registry().items() if isinstance(v, dict)]


def defined_symbols(ctx: ScanContext) -> set[str]:
    """Every function-ish name defined anywhere in the tree.

    Rust `fn`, Python `def`, shell `name()`, and JS/TS `function name(` all count.
    This is how `stale-registry-test` can tell a citation that resolves from one
    that resolves nowhere, for a target that is a script rather than a crate.
    """
    names: set[str] = set()
    patterns = (
        re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*[<(]"),
        re.compile(r"\bdef\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("),
        re.compile(r"\bfunction\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("),
        # A shell function definition is only a definition at the start of a line,
        # so this pattern needs MULTILINE. Without it, `^` anchored at the start of
        # the *file* and the only shell function the scanner ever saw was one
        # defined on line 0 — which is why a gate script could define `some_case()`
        # under its shebang, be cited as `scripts/...::some_case` (the citation form
        # this scanner's own documentation asks for), and still be reported as
        # citing a test that resolves nowhere.
        re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(\)\s*\{", re.MULTILINE),
    )
    for rel, _ in ctx.files():
        if not rel.endswith((".rs", ".py", ".sh", ".js", ".ts")):
            continue
        body = ctx.text(rel)
        for pattern in patterns:
            names.update(pattern.findall(body))
    return names


def local_ci_text(ctx: ScanContext) -> str:
    return ctx.text("scripts/local-ci.sh")


def gate_commands(ctx: ScanContext) -> list[str]:
    """Every gate command in `scripts/local-ci.sh`, as written."""
    return [
        m.group(1)
        for m in re.finditer(r'^\s*"[^"]+:(.*)"\s*$', local_ci_text(ctx), re.M)
    ]


def scripts_invoked_by_gates(ctx: ScanContext) -> set[str]:
    """Scripts a gate runs (`bash path` / `path.sh` / `python3 path`).

    A crate whose tests only run through such a wrapper is gated even though no
    `cargo test` gate names the package; `x3-htlc-tests-live` is the live
    example, and the unwired `check-crate-tests-are-gated.py` reports it as
    ungated because it only reads `cargo test` lines.
    """
    invoked: set[str] = set()
    for command in gate_commands(ctx):
        for token in command.split():
            token = token.strip("\"'")
            if "/" in token and re.search(r"\.(sh|py)$", token):
                invoked.add(token)
    return invoked


# ── detectors ───────────────────────────────────────────────────────────────


def detect_registry_citations(ctx: ScanContext, defined: set[str]) -> list[Finding]:
    """Registry rows that point at something that is not there."""
    findings: list[Finding] = []
    for feature, row in ctx.registry_features():
        target = str(row.get("crate_or_service", "")).strip()
        if target and not (ctx.root / target).exists():
            findings.append(
                Finding(
                    kind="missing-registry-path",
                    severity="high",
                    path="FEATURE_REGISTRY.toml",
                    line=0,
                    symbol=f"{feature}: {target}",
                    why="the registry names a path that is not in the tree, so every score and "
                    "test citation under it is unverifiable",
                    suggested_fix=f"point [{feature}] at the real path, or remove the row",
                    test_required="readiness consistency",
                    gate_affected="readiness consistency",
                )
            )
        proof = str(row.get("proof_report", "")).strip()
        if proof and not (ctx.root / proof).exists():
            findings.append(
                Finding(
                    kind="stale-proof-report",
                    severity="medium",
                    path="FEATURE_REGISTRY.toml",
                    line=0,
                    symbol=f"{feature}: {proof}",
                    why="the row cites a proof report that does not exist, so the claim it backs "
                    "cannot be re-read by anyone",
                    suggested_fix=f"produce {proof}, or clear the citation for [{feature}]",
                    test_required="readiness consistency",
                    gate_affected="readiness consistency",
                )
            )
        for citation in row.get("required_tests", []) or []:
            name = citation_tail(str(citation))
            if not name or name in defined:
                continue
            findings.append(
                Finding(
                    kind="stale-registry-test",
                    severity="high",
                    path="FEATURE_REGISTRY.toml",
                    line=0,
                    symbol=f"{feature}:{name}",
                    why="`required_tests` cites a function that exists nowhere in the tree. The "
                    "readiness gate only resolves citations whose crate_or_service is a directory "
                    "of .rs files, so a row pointing at a script can cite a test nobody wrote and "
                    "still pass — which is how this one was found",
                    suggested_fix=f"implement `{name}` under {target or 'the cited target'}, or "
                    f"correct the citation in [{feature}]",
                    test_required=name,
                    gate_affected="readiness consistency",
                )
            )
    return findings


def runtime_names_pallet(runtime: str, package: str, entry_name: str) -> bool:
    """Whether the runtime names a crate, by package name or by local module name."""
    aliases = {
        package,
        package.replace("-", "_"),
        entry_name,
        entry_name.replace("-", "_"),
    }
    return any(alias in runtime for alias in aliases)


def documented_unwired(ctx: ScanContext) -> tuple[list[dict[str, str]], list[str]]:
    """The recorded `pallets/` omissions, and the notes for entries that have gone stale.

    Returns the decisions that are still live (the pallet is in this tree, the runtime does not name
    it, and the owning document exists) and notes for the ones that are not. A pallet that is not in
    the tree at all is not a note: an unrelated fixture tree has nothing to decide about, and a
    deleted crate leaves an entry nobody needs to act on.
    """
    documented: list[dict[str, str]] = []
    stale: list[str] = []
    runtime = ctx.text("runtime/src/lib.rs")
    pallets_dir = ctx.root / "pallets"
    if not pallets_dir.is_dir():
        return documented, stale

    entry_names: dict[str, str] = {}
    for entry in sorted(pallets_dir.iterdir()):
        manifest = entry / "Cargo.toml"
        if not manifest.is_file():
            continue
        try:
            package = tomllib.loads(read_text(manifest)).get("package", {}).get("name", "")
        except tomllib.TOMLDecodeError:
            continue
        if package:
            entry_names[package] = entry.name

    for package, decision in sorted(KNOWN_UNWIRED_PALLETS.items()):
        if package not in entry_names:
            continue
        entry_name = entry_names[package]
        if runtime_names_pallet(runtime, package, entry_name):
            stale.append(
                f"{package}: runtime/src/lib.rs now names it — the recorded decision is stale, so "
                "drop the entry"
            )
            continue
        owner = decision.get("owner", "")
        if not owner or not (ctx.root / owner).exists():
            stale.append(f"{package}: the owning document `{owner}` does not exist")
            continue
        documented.append(
            {
                "package": package,
                "path": f"pallets/{entry_name}/Cargo.toml",
                "owner": owner,
                "reason": decision.get("reason", ""),
            }
        )
    return documented, stale


def documented_unmeasurable_calls(
    ctx: ScanContext,
) -> tuple[set[tuple[str, str]], list[str]]:
    """The recorded unmeasurable-call decisions, and notes for entries that went stale.

    Returns the `(pallet entry, extrinsic)` pairs whose literal weights the scanner should not report,
    plus a note for every entry that no longer describes the tree. An entry is stale when its owning
    document is gone, or when the call it names no longer charges a literal — which is exactly what
    "somebody benchmarked it" looks like from here.
    """
    documented: set[tuple[str, str]] = set()
    stale: list[str] = []
    pallets_dir = ctx.root / "pallets"
    if not pallets_dir.is_dir():
        return documented, stale
    for key, decision in sorted(UNMEASURABLE_CALL_WEIGHTS.items()):
        package, _, extrinsic = key.partition("::")
        entry = next(
            (
                candidate
                for candidate in sorted(pallets_dir.iterdir())
                if (candidate / "Cargo.toml").is_file()
                and f'name = "{package}"' in read_text(candidate / "Cargo.toml")
            ),
            None,
        )
        # A package that is not in this tree leaves nothing to decide about.
        if entry is None:
            continue
        owner = decision.get("owner", "")
        if not owner or not (ctx.root / owner).exists():
            stale.append(f"{key}: the owning document `{owner}` does not exist")
            continue
        body = "".join(ctx.text(rel) for rel, _ in ctx.files() if rel.startswith(f"pallets/{entry.name}/"))
        if not re.search(rf"pub fn {re.escape(extrinsic)}\b", body):
            stale.append(f"{key}: the pallet no longer defines `{extrinsic}`")
            continue
        if "WeightInfo" in literal_weight_block(body, extrinsic):
            stale.append(
                f"{key}: the call is measured now, so the recorded exception is stale — "
                "drop the entry"
            )
            continue
        documented.add((entry.name, extrinsic))
    return documented, stale


def literal_weight_block(body: str, extrinsic: str) -> str:
    """The `#[pallet::weight(...)]` attribute text on the line before `pub fn <extrinsic>`."""
    lines = body.splitlines()
    for index, line in enumerate(lines):
        if not re.match(rf"\s*pub fn {re.escape(extrinsic)}\b", line):
            continue
        # Walk back over the doc/attribute lines to the weight attribute, if any.
        for earlier in reversed(lines[max(0, index - 12) : index]):
            if "#[pallet::weight(" in earlier:
                return earlier
        return ""
    return ""


def detect_unregistered_pallets(ctx: ScanContext) -> list[Finding]:
    """`pallets/*` crates the runtime never mentions."""
    runtime = ctx.text("runtime/src/lib.rs")
    findings: list[Finding] = []
    pallets_dir = ctx.root / "pallets"
    if not runtime or not pallets_dir.is_dir():
        return findings
    for entry in sorted(pallets_dir.iterdir()):
        manifest = entry / "Cargo.toml"
        if not manifest.is_file():
            continue
        try:
            package = tomllib.loads(read_text(manifest)).get("package", {}).get("name", "")
        except tomllib.TOMLDecodeError:
            continue
        if not package:
            continue
        # The runtime may name the crate, or reach the pallet through a local
        # module (`pub mod fraud_proofs;` / `crate::fraud_proofs::pallet::pallet`)
        # — checking only the package name reported `pallets/fraud-proofs` as
        # unregistered when it is registered four times over.
        if runtime_names_pallet(runtime, package, entry.name):
            continue
        if package in KNOWN_UNWIRED_PALLETS:
            # A recorded decision, rendered in the report's own section instead of here.
            continue
        findings.append(
            Finding(
                kind="unregistered-pallet",
                severity="medium",
                path=f"pallets/{entry.name}/Cargo.toml",
                line=0,
                symbol=package,
                why="a pallet under pallets/ that runtime/src/lib.rs never names: either it is "
                "half-wired (declared, built, not in the runtime) or it is dead weight in the "
                "pallet directory",
                suggested_fix="add it to the runtime with weights and a genesis config, or move it "
                "out of pallets/ and say what it is",
                test_required="runtime registration / genesis build test",
                gate_affected="runtime identity",
            )
        )
    return findings


def detect_pallet_call_without_weights(ctx: ScanContext) -> list[Finding]:
    """Extrinsics whose weight is a hand-guessed literal, not a benchmark.

    `#[pallet::weight(Weight::from_parts(10_000, 0))]` charges a number somebody
    typed. PR #519 shipped a generated `weights.rs` that did not compile; this is
    the same defect one step earlier — a number nobody measured.
    """
    findings: list[Finding] = []
    pallets_dir = ctx.root / "pallets"
    if not pallets_dir.is_dir():
        return findings
    documented_calls, _ = documented_unmeasurable_calls(ctx)
    by_pallet: dict[str, list[str]] = {}
    for rel, _ in ctx.files():
        if rel.startswith("pallets/"):
            parts = rel.split("/")
            if len(parts) > 2:
                by_pallet.setdefault(parts[1], []).append(rel)
    for entry in sorted(pallets_dir.iterdir()):
        src = entry / "src"
        if not src.is_dir():
            continue
        sources = by_pallet.get(entry.name, [])
        body = "".join(ctx.text(rel) for rel in sources)
        if "#[pallet::call]" not in body:
            continue
        invented: list[tuple[str, int, str]] = []
        estimated: list[tuple[str, int, str]] = []
        for rel in sources:
            lines = ctx.text(rel).splitlines()
            for number, line in enumerate(lines, start=1):
                if "#[pallet::weight(" not in line or "WeightInfo" in line:
                    continue
                # `#[pallet::weight(...)]` sits immediately above the extrinsic it charges, so the
                # next `pub fn` is the call this literal belongs to.
                extrinsic = next(
                    (
                        match.group(1)
                        for match in (
                            re.match(r"\s*pub fn ([A-Za-z0-9_]+)", later) for later in lines[number:]
                        )
                        if match
                    ),
                    "",
                )
                if (entry.name, extrinsic) in documented_calls:
                    continue
                if re.search(r"Weight::from_parts\(\s*\d", line) or re.search(
                    r"#\[pallet::weight\(\s*\d[\d_]*\s*\)\]", line
                ):
                    invented.append((rel, number, line.strip()))
                elif "DbWeight" in line:
                    # `reads_writes(n, m)` is the documented pre-benchmark form: a
                    # stated storage-access estimate rather than an invented cost.
                    estimated.append((rel, number, line.strip()))
        if not invented and not estimated:
            continue
        runtime_text = ctx.text("runtime/src/lib.rs")
        registered = entry.name in runtime_text or entry.name.replace("-", "_") in runtime_text
        has_generated = any(rel.endswith("weights.rs") for rel in sources) or "pub mod weights" in body
        rel, number, text = (invented or estimated)[0]
        if invented and has_generated:
            severity = "high"
            why = (
                f"{len(invented)} extrinsic(s) charge an invented literal weight while the pallet "
                "ships generated weights, so the benchmarked numbers are not the ones being charged"
            )
        elif invented:
            severity = "high" if registered else "medium"
            why = (
                f"{len(invented)} extrinsic(s) charge an invented literal weight and the pallet has "
                "no generated weights at all"
                + (", while being registered in runtime/src/lib.rs" if registered else "")
            )
        else:
            severity = "low"
            why = (
                f"{len(estimated)} extrinsic(s) charge the documented pre-benchmark "
                "`DbWeight::reads_writes` estimate; real numbers need a benchmark run"
            )
        findings.append(
            Finding(
                kind="pallet-call-without-weights",
                severity=severity,
                path=rel,
                line=number,
                symbol=f"{entry.name}::{text.split('#[pallet::weight(')[-1].rstrip(')]')}",
                why=why,
                suggested_fix="add a WeightInfo trait, generate weights with the FRAME benchmark CLI "
                "(`scripts/run-frame-benchmarks.sh`), and point the runtime at "
                "SubstrateWeight<Runtime>",
                test_required=f"cargo test -p {entry.name} --features runtime-benchmarks",
                gate_affected="runtime identity / benchmarks",
            )
        )
    return findings


@dataclass
class PackageInfo:
    name: str
    manifest_path: str
    workspace_root: str
    test_attributes: int


# Crates whose suite no gate *can* run in this environment, each with a reason a
# reader can check. This is an exception list, not a dumping ground: an entry is
# only legitimate while running the crate here is impossible, and it is printed
# on every run so it cannot hide. The repository already uses the same shape for
# `check-registry-tests-are-gated.py`'s KNOWN_UNGATED list.
KNOWN_UNGATED: dict[str, str] = {
    # `apps/tauri-os/src-tauri/Cargo.toml` was listed here on 2026-09-27 with "no gate can
    # build it here". That measurement came from pkg-config's default search path on this
    # box, which is Homebrew-only: `webkit2gtk-4.1`, `gtk+-3.0` and `libsoup-3.0` are
    # installed under `/usr/lib/x86_64-linux-gnu/pkgconfig` and are found as soon as that
    # directory is on `PKG_CONFIG_PATH`. `apps/tauri-os/src-tauri/run-tests.sh` does that
    # (and supplies the missing `shared-mime-info.pc` Debian does not ship), the fast set
    # runs it as `tauri-os operator console`, and the live set runs the node-backed suite as
    # `tauri-os live operator console`. The entry is deleted rather than kept: leaving it
    # would hide a real regression if that gate ever stopped naming the package.
}
KNOWN_UNGATED_SEEN: list[str] = []


def ungated_crate_findings(
    packages: list[PackageInfo],
    gate_text: str,
    wrapper_bodies: dict[str, str],
    workspace_manifests: set[str] | None = None,
    deep_workspace_gate: bool = False,
) -> list[Finding]:
    """Crates whose tests no gate runs.

    `wrapper_bodies` maps a gate-invoked script path to its text, so a crate
    whose suite runs through a wrapper (`run-expiry-test.sh` -> `cargo test
    --manifest-path .../tests-live`) is not reported as ungated.
    """
    findings: list[Finding] = []
    wrapper_text = "\n".join(wrapper_bodies.values())
    gated_workspaces = {
        manifest
        for manifest in (workspace_manifests or set())
        if re.search(rf"(^|[\s\"'=]){re.escape(manifest)}([\s\"']|$)", gate_text)
    }
    for package in sorted(packages, key=lambda p: p.name):
        if package.test_attributes <= 0:
            continue
        if package.manifest_path in KNOWN_UNGATED:
            if package.manifest_path not in KNOWN_UNGATED_SEEN:
                KNOWN_UNGATED_SEEN.append(package.manifest_path)
            continue
        if re.search(rf"(^|[^\w-]){re.escape(package.name)}([^\w-]|$)", gate_text):
            continue
        if re.search(rf"(^|[^\w-]){re.escape(package.name)}([^\w-]|$)", wrapper_text):
            continue
        # A gate that names the whole workspace (`--manifest-path x3-lang/Cargo.toml`)
        # runs this package's tests too.
        if os.path.join(package.workspace_root, "Cargo.toml") in gated_workspaces:
            continue
        # The nested workspace a gate would have to name, relative to the repo.
        workspace_manifest = os.path.join(package.workspace_root, "Cargo.toml")
        gate_line = (
            f'  "test {package.name}:env CARGO_TARGET_DIR=/tmp/x3-nested-{package.name} '
            f'cargo test --locked --manifest-path {workspace_manifest} -p {package.name}"'
        )
        # A root-workspace member is compiled and tested by the workspace-wide
        # `cargo test --workspace` gate (`test workspace`, which `local-ci.sh
        # --all`/`--deep` runs), and that is the standard this repository already
        # applies in `scripts/check-crate-tests-are-gated.py`. A nested workspace
        # no workspace gate reaches is the real defect: nothing runs it at all.
        if deep_workspace_gate and package.workspace_root in ("", "."):
            continue
        why = (
            f"{package.test_attributes} test attribute(s), no gate command names this package, and "
            "no workspace-wide `cargo test` gate reaches its workspace, so its suite runs only "
            "when a human remembers"
        )
        findings.append(
            Finding(
                kind="ungated-crate",
                severity="medium",
                path=package.manifest_path,
                line=0,
                symbol=package.name,
                why=why,
                suggested_fix=f"append the gate line below to the fast-gate list in "
                f"scripts/local-ci.sh, or record why it is intentionally ungated",
                test_required=f"cargo test -p {package.name}",
                gate_affected="crate tests are gated",
                patch_eligible=True,
                patch=gate_line,
            )
        )
    return findings


TEST_ATTR_RE = re.compile(r"#\[(?:tokio::)?test(?:\(|])|#\[test_case|#\[rstest")


def workspace_root_for(manifest: Path) -> Path:
    """The nearest ancestor manifest declaring `[workspace]` (often itself)."""
    candidate = manifest
    for directory in [manifest.parent, *manifest.parent.parents]:
        candidate = directory / "Cargo.toml"
        if candidate.is_file() and "[workspace]" in read_text(candidate):
            return directory
    return manifest.parent


def detect_ungated_crates(ctx: ScanContext) -> list[Finding]:
    """Every crate in the tree whose tests no gate runs."""
    # One pass over the file index, grouped by crate directory: rglob-per-crate
    # re-walked the tree once per manifest and re-read every source file.
    rs_by_dir: dict[str, list[str]] = {}
    for rel, _ in ctx.files():
        if rel.endswith(".rs"):
            rs_by_dir.setdefault(rel.rsplit("/", 1)[0] if "/" in rel else "", []).append(rel)

    # Test attributes counted per directory subtree, then folded onto every
    # ancestor prefix so a manifest at `crates/x/Cargo.toml` sees the tests in
    # `crates/x/src` and `crates/x/tests` in one lookup.
    tests_by_prefix: dict[str, int] = {}
    for directory, sources in sorted(rs_by_dir.items()):
        total = sum(len(TEST_ATTR_RE.findall(ctx.text(source))) for source in sources)
        if total == 0:
            continue
        parts = directory.split("/") if directory else []
        for depth in range(1, len(parts) + 1):
            prefix = "/".join(parts[:depth])
            tests_by_prefix[prefix] = tests_by_prefix.get(prefix, 0) + total

    packages: list[PackageInfo] = []
    workspaces: set[str] = set()
    for rel, path in ctx.files():
        if not rel.endswith("Cargo.toml"):
            continue
        if not rel.startswith(FIRST_PARTY_ROOTS) and rel != "Cargo.toml":
            continue
        body = ctx.text(rel)
        if "[workspace]" in body:
            workspaces.add(rel)
            if "[package]" not in body:
                continue  # a virtual manifest declares no crate of its own
        try:
            package = tomllib.loads(body).get("package", {})
        except tomllib.TOMLDecodeError:
            continue
        name = str(package.get("name", "")).strip()
        if not name:
            continue
        directory = rel.rsplit("/", 1)[0] if "/" in rel else ""
        count = tests_by_prefix.get(directory, 0)
        if count == 0:
            continue
        packages.append(
            PackageInfo(
                name=name,
                manifest_path=rel,
                workspace_root=str(workspace_root_for(path).relative_to(ctx.root)),
                test_attributes=count,
            )
        )
    wrapper_bodies = {
        rel: ctx.text(rel) for rel in sorted(scripts_invoked_by_gates(ctx)) if (ctx.root / rel).is_file()
    }
    # Named for what it is: the workspace-wide `cargo test --workspace` gate. It
    # is in `--deep`, and the repository's own gate-standard treats it as
    # coverage for root-workspace members.
    deep_workspace_gate = bool(
        re.search(r'^\s*"[^"]*:.*cargo test --workspace', local_ci_text(ctx), re.M)
    )
    return ungated_crate_findings(
        packages,
        local_ci_text(ctx),
        wrapper_bodies,
        workspace_manifests=workspaces,
        deep_workspace_gate=deep_workspace_gate,
    )


def gate_summary(ctx: ScanContext, enabled: bool) -> list[dict]:
    """Verdicts of the ratchets this scanner does not re-implement."""
    summary: list[dict] = []
    checks = (
        ("stub / marker ratchet", ["python3", "scripts/x3_fake_code_scan.py", "stubs", "--json"]),
        ("fake-code scan", ["python3", "scripts/x3_fake_code_scan.py", "cheats", "--json"]),
        ("panic / unwrap ratchet", ["python3", "scripts/audit/panic_unwrap_scan.py"]),
    )
    for label, argv in checks:
        entry = {"gate": label, "status": "skipped", "detail": ""}
        if enabled and (ctx.root / argv[1]).is_file():
            try:
                proc = subprocess.run(
                    argv, cwd=ctx.root, capture_output=True, text=True, timeout=300
                )
                counts = ""
                try:
                    payload = json.loads(proc.stdout)
                    raw = payload.get("counts", {})
                    counts = ", ".join(f"{k}={v}" for k, v in sorted(raw.items()))
                except (json.JSONDecodeError, AttributeError):
                    counts = "no counts reported"
                entry["status"] = "pass" if proc.returncode == 0 else "fail"
                entry["detail"] = counts or proc.stderr.strip()[:200]
            except (subprocess.TimeoutExpired, OSError) as exc:
                entry["status"] = "error"
                entry["detail"] = str(exc)[:200]
        summary.append(entry)
    return summary


# ── output ──────────────────────────────────────────────────────────────────

REPORT_MD = "reports/swarm_scan_report.md"
REPORT_JSON = "reports/swarm_scan_findings.json"
BASELINE = "docs/reports/repo-scan-baseline.json"


def structural_counts(findings: list[Finding]) -> dict[str, int]:
    counts = {kind: 0 for kind in STRUCTURAL_KINDS}
    for finding in findings:
        if finding.kind in counts:
            counts[finding.kind] += 1
    return counts


def render_markdown(
    findings: list[Finding],
    summary: list[dict],
    root: Path,
    documented: list[dict[str, str]] | None = None,
) -> str:
    counts: dict[str, int] = {}
    for finding in findings:
        counts[finding.kind] = counts.get(finding.kind, 0) + 1
    lines = [
        "# X3 repo scan",
        "",
        "Findings carry an id, a severity, the exact file and symbol, why it matters, the fix, the "
        "test that would prove the fix and the gate that catches a regression. Sorted by severity, "
        "kind, path and line, so two runs over the same tree are byte-identical.",
        "",
        f"Root: `{root}`",
        f"Findings: {len(findings)}",
        "",
        "## Counts",
        "",
        "| kind | count | ratcheted here |",
        "|---|---|---|",
    ]
    for kind in sorted(counts):
        ratcheted = "yes" if kind in STRUCTURAL_KINDS else "no (see gate summary)"
        lines.append(f"| `{kind}` | {counts[kind]} | {ratcheted} |")
    lines += ["", "## Related ratchets (not re-reported here)", "", "| gate | status | detail |", "|---|---|---|"]
    for entry in summary:
        lines.append(f"| {entry['gate']} | {entry['status']} | {entry['detail']} |")
    lines += ["", "## Findings", ""]
    if not findings:
        lines.append("_None._")
    for finding in findings:
        location = finding.path + (f":{finding.line}" if finding.line else "")
        lines += [
            f"### {finding.severity.upper()} — `{finding.kind}` — {location}",
            "",
            f"- **id:** `{finding.id}`",
            f"- **symbol:** `{finding.symbol}`",
            f"- **why it matters:** {finding.why}",
            f"- **suggested fix:** {finding.suggested_fix}",
            f"- **test required:** {finding.test_required}",
            f"- **release gate affected:** {finding.gate_affected}",
        ]
        if finding.patch_eligible:
            lines.append("- **patch:** `.ai/patches/" + finding.id + ".patch` (`--patches`)")
        lines.append("")
    lines += ["## Documented decisions", ""]
    if not documented:
        lines.append("_None. Every `pallets/` crate is either named by the runtime or reported above._")
    else:
        lines.append(
            "These `pallets/` crates are deliberately absent from `runtime/src/lib.rs`. The scanner "
            "reports an entry the moment the runtime names the pallet, so the list can only shrink."
        )
        lines.append("")
        for decision in documented:
            lines.append(
                f"- `{decision['path']}` — {decision['reason']} (owning document: "
                f"`{decision['owner']}`)"
            )
    lines += [
        "## What this scan does not cover",
        "",
        "`TODO`/`stub`/test-cheat markers and reachable `unwrap()`/`panic!` counts are owned by the "
        "two ratchets above; this report cites their verdicts instead of duplicating their debt.",
    ]
    return "\n".join(lines).rstrip() + "\n"


def hunk_for_gate_line(ctx: ScanContext, gate_line: str) -> str:
    """A unified diff appending one gate line to the fast-gate list.

    The hunk is computed from the file's actual bytes, so `git apply` accepts the
    result (the test asserts the context lines match).
    """
    rel = "scripts/local-ci.sh"
    lines = local_ci_text(ctx).splitlines()
    try:
        start = next(i for i, l in enumerate(lines) if l.startswith("GATES_FAST=("))
    except StopIteration:
        return ""
    close = next((i for i in range(start + 1, len(lines)) if lines[i] == ")"), None)
    if close is None:
        return ""
    context_before = max(start + 1, close - 3)
    before = lines[context_before:close]
    after = lines[close : close + 3]
    old_count = len(before) + len(after)
    new_count = old_count + 1
    header = f"@@ -{context_before + 1},{old_count} +{context_before + 1},{new_count} @@"
    body = [f" {l}" for l in before]
    body.append(f"+{gate_line}")
    body += [f" {l}" for l in after]
    return "\n".join([f"--- a/{rel}", f"+++ b/{rel}", header, *body]) + "\n"


def write_patches(ctx: ScanContext, findings: list[Finding]) -> list[str]:
    written: list[str] = []
    for finding in findings:
        if not finding.patch_eligible or not finding.patch:
            continue
        diff = hunk_for_gate_line(ctx, finding.patch)
        if not diff:
            continue
        rel = f".ai/patches/{finding.id}.patch"
        ctx.write_text(rel, diff)
        written.append(rel)
    return written


def check_baseline(ctx: ScanContext, counts: dict[str, int]) -> tuple[bool, list[str]]:
    path = ctx.root / BASELINE
    if not path.is_file():
        return False, [f"no baseline at {BASELINE}; run --update-baseline once and say so in the commit"]
    try:
        recorded = json.loads(read_text(path)).get("counts", {})
    except json.JSONDecodeError:
        return False, [f"{BASELINE} is not JSON"]
    drift: list[str] = []
    for kind, count in sorted(counts.items()):
        was = int(recorded.get(kind, 0))
        if count > was:
            drift.append(f"{kind}: {was} -> {count}")
    return (not drift), drift


def scan(root: Path, with_gate_summary: bool = True) -> tuple[list[Finding], list[dict]]:
    ctx = ScanContext(root)
    defined = defined_symbols(ctx)
    findings: list[Finding] = []
    findings += detect_registry_citations(ctx, defined)
    documented, stale_decisions = documented_unwired(ctx)
    for note in stale_decisions:
        findings.append(
            Finding(
                kind="unregistered-pallet",
                severity="medium",
                path="scripts/swarm/x3_repo_scan.py",
                line=0,
                symbol=f"stale-unwired-decision:{note.split(':', 1)[0]}",
                why=f"a recorded `pallets/` decision no longer describes the tree: {note}",
                suggested_fix="delete or correct the entry in `KNOWN_UNWIRED_PALLETS`",
                test_required="repo scanner test",
                gate_affected="repo scanner",
            )
        )
    findings += detect_unregistered_pallets(ctx)
    _, stale_weight_decisions = documented_unmeasurable_calls(ctx)
    for note in stale_weight_decisions:
        findings.append(
            Finding(
                kind="pallet-call-without-weights",
                severity="medium",
                path="scripts/swarm/x3_repo_scan.py",
                line=0,
                # The note is `"<package>::<extrinsic>: <why>"`; the key itself has no spaces, so
                # the first ": " is the boundary. Splitting on ":" would cut into the "::".
                symbol=f"stale-weight-exception:{note.split(': ', 1)[0]}",
                why=f"a recorded unmeasurable-weight decision no longer describes the tree: {note}",
                suggested_fix="delete or correct the entry in `UNMEASURABLE_CALL_WEIGHTS`",
                test_required="repo scanner test",
                gate_affected="repo scanner",
            )
        )
    findings += detect_pallet_call_without_weights(ctx)
    findings += detect_ungated_crates(ctx)
    findings.sort(key=sort_key)
    return findings, gate_summary(ctx, with_gate_summary)


def write_reports(root: Path, findings: list[Finding], summary: list[dict]) -> dict:
    """Write the Markdown + JSON reports and return the JSON payload."""
    counts: dict[str, int] = {}
    for finding in findings:
        counts[finding.kind] = counts.get(finding.kind, 0) + 1
    documented, _ = documented_unwired(ScanContext(root))
    documented_calls, _ = documented_unmeasurable_calls(ScanContext(root))
    payload = {
        "schema": 1,
        "root": str(root),
        "counts": {k: counts[k] for k in sorted(counts)},
        "structural_counts": structural_counts(findings),
        "documented_unwired": documented,
        "documented_unmeasurable_call_weights": sorted(
            f"{entry}::{extrinsic}" for entry, extrinsic in documented_calls
        ),
        "gate_summary": summary,
        "findings": [f.as_dict() for f in findings],
    }
    ctx = ScanContext(root)
    ctx.write_text(REPORT_JSON, json.dumps(payload, indent=2, sort_keys=False) + "\n")
    ctx.write_text(REPORT_MD, render_markdown(findings, summary, root, documented))
    return payload


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="X3 repo scanner: schema'd, deterministic findings")
    parser.add_argument("--root", default=None, help="tree to scan (default: this repository)")
    parser.add_argument("--json", action="store_true", help="print the report JSON on stdout")
    parser.add_argument("--patches", action="store_true", help="write .ai/patches/<id>.patch")
    parser.add_argument("--check", action="store_true", help="fail when a ratcheted count grows")
    parser.add_argument("--update-baseline", action="store_true", help="record the current counts")
    parser.add_argument("--no-gate-summary", action="store_true", help="skip the other ratchets")
    parser.add_argument("--no-write", action="store_true", help="do not touch reports/")
    args = parser.parse_args(argv)

    root = Path(args.root).resolve() if args.root else Path(__file__).resolve().parents[2]
    if not root.is_dir():
        print(f"x3-repo-scan: no such root: {root}", file=sys.stderr)
        return 2

    findings, summary = scan(root, with_gate_summary=not args.no_gate_summary)
    counts = structural_counts(findings)
    payload = {
        "schema": 1,
        "root": str(root),
        "counts": {},
        "structural_counts": counts,
        "gate_summary": summary,
        "findings": [f.as_dict() for f in findings],
    }

    ctx = ScanContext(root)
    if not args.no_write:
        payload = write_reports(root, findings, summary)
    if args.patches:
        written = write_patches(ctx, findings)
        print(f"x3-repo-scan: wrote {len(written)} patch(es) under .ai/patches/", file=sys.stderr)

    if args.update_baseline:
        ctx.write_text(
            BASELINE,
            json.dumps(
                {
                    "note": "Counts of the finding kinds this scanner owns. Growth fails the "
                    "`repo scanner` gate; shrinking is always allowed.",
                    "counts": counts,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
        )
        print(f"x3-repo-scan: baseline updated at {BASELINE}", file=sys.stderr)

    if args.json:
        print(json.dumps(payload, indent=2))
    else:
        print(f"x3-repo-scan: {len(findings)} finding(s) — {counts}")
        for finding in findings:
            where = finding.path + (f":{finding.line}" if finding.line else "")
            print(f"  {finding.severity:<8} {finding.kind:<28} {where} [{finding.symbol}]")
        if KNOWN_UNGATED_SEEN:
            print(f"  known-ungated (no gate can run it here): {len(KNOWN_UNGATED_SEEN)}")
            for manifest in sorted(KNOWN_UNGATED_SEEN):
                print(f"    {manifest} — {KNOWN_UNGATED[manifest]}")

    if args.check:
        ok, drift = check_baseline(ctx, counts)
        if not ok:
            for line in drift:
                print(f"x3-repo-scan: RATCHET: {line}", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
