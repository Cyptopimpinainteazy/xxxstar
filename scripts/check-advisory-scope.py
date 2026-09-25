#!/usr/bin/env python3
"""Check resolved versions and feature preconditions against verified advisories.

GitHub's advisory metadata is sometimes coarser than the regression that actually
produced the bug, and an advisory's stated precondition is sometimes a *feature*
rather than a version. Three cases have come up in this tree, and each gets its
own enforceable record kind:

  not_affected   no resolved version of the package falls inside the window that
                 is really vulnerable (GHSA-vxx9-2994-q338: the yamux advisory
                 publishes `<0.13.10`, but the guard-ordering defect only ever
                 existed in 0.13.9).

  unreachable    a resolved version *is* inside the published window, but the
                 advisory states a build-time precondition that this graph does
                 not meet, so the vulnerable code is not compiled
                 (GHSA-3v94-mw7p-v465: the NSEC3 loop lives in
                 `DnssecDnsHandle`, behind `#[cfg(feature = "dnssec*")]`, and no
                 dnssec feature is enabled anywhere here).

  accepted_risk  a resolved version is inside the window and the code is built;
                 the record carries the reason, and the check fails when the
                 vulnerable version disappears -- so the acceptance cannot
                 outlive the exposure it describes
                 (GHSA-q2qq-hmj6-3wpp).

Records also cross-check that their `rustsec` id is ignored in *both* ignore
lists. `.cargo/audit.toml` and `deny.toml` each carry one, and the config itself
says to keep them in sync; nothing enforced that until now.

Usage:
  scripts/check-advisory-scope.py            # check, non-zero on drift
  scripts/check-advisory-scope.py --list     # print the records and exit
"""

from __future__ import annotations

import argparse
import os
import pathlib
import re
import subprocess
import sys

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - 3.10 and older
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        sys.exit(
            "check-advisory-scope needs a TOML parser: Python 3.11+ (tomllib) "
            "or the `tomli` backport"
        )

ROOT = pathlib.Path(__file__).resolve().parent.parent
RECORDS = ROOT / "security" / "advisory-scope.toml"
AUDIT_CONFIGS = (ROOT / ".cargo" / "audit.toml", ROOT / "deny.toml")

# Directories that hold build output, vendored crate sources, or another agent's
# checkout rather than this tree's own manifests. Hidden directories are skipped
# wholesale: `.git`, `.wt-*` agent worktrees, `.kilo/worktrees`, and
# `.pre-edit-snapshot` all live there, and none of them is part of what a release
# builds.
SKIP_DIRS = {"target", "node_modules", "vendor"}

STATUSES = {"not_affected", "unreachable", "accepted_risk"}

# Advisories use pre-release bounds (`>=0.25.0-alpha.3`). The numeric part is what
# decides every comparison in this tree, because no resolved version here carries a
# pre-release tag; `main` refuses to compare at all if one ever does, rather than
# silently ordering it wrong.
_COMPARATOR = re.compile(r"^(>=|<=|>|<|=)?\s*(\d+(?:\.\d+)*)(?:-[0-9A-Za-z.-]+)?$")
_FEATURE = re.compile(r'feature "([^"]+)"')


def _is_skipped(name):
    return name.startswith(".") or name in SKIP_DIRS or name.endswith("-vendor")


def lockfiles():
    """Every Cargo.lock in the tree except build output and vendored sources.

    A plain `rglob` would descend into `target/` and `node_modules/`, which is
    tens of thousands of stat calls for a gate that wants to be instant, so the
    excluded directories are pruned during the walk instead.
    """
    found = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = sorted(name for name in dirnames if not _is_skipped(name))
        if "Cargo.lock" in filenames:
            found.append(pathlib.Path(dirpath) / "Cargo.lock")
    return sorted(found)


def parse_lock(text):
    """Minimal [[package]] reader: (name, version) pairs, in file order."""
    packages = []
    name = None
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("["):
            name = None
        elif line.startswith("name = "):
            name = line[len("name = "):].strip().strip('"')
        elif line.startswith("version = ") and name is not None:
            packages.append((name, line[len("version = "):].strip().strip('"')))
            name = None
    return packages


def parse_version(raw):
    core = raw.split("+")[0].split("-")[0]
    return tuple(int(part) for part in core.split("."))


def _padded(a, b):
    width = max(len(a), len(b))
    return a + (0,) * (width - len(a)), b + (0,) * (width - len(b))


def satisfies(version, spec):
    """True when `version` meets every comma-separated comparator in `spec`."""
    for part in spec.split(","):
        part = part.strip()
        match = _COMPARATOR.match(part)
        if not match:
            raise ValueError("unsupported version range component: %r" % (part,))
        operator = match.group(1) or "="
        left, right = _padded(version, parse_version(match.group(2)))
        ok = {
            ">=": left >= right,
            ">": left > right,
            "<=": left <= right,
            "<": left < right,
            "=": left == right,
        }[operator]
        if not ok:
            return False
    return True


def feature_names(tree_text):
    """Enabled feature names in `cargo tree -e features` output."""
    return set(_FEATURE.findall(tree_text))


def missing_features(tree_text, required):
    return sorted(name for name in required if name not in feature_names(tree_text))


def present_features(tree_text, forbidden):
    return sorted(name for name in forbidden if name in feature_names(tree_text))


def enabled_feature_tree():
    """`cargo tree -e features --workspace` output, or an error string."""
    try:
        completed = subprocess.run(
            ["cargo", "tree", "-e", "features", "--workspace", "--offline"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
    except OSError as error:  # cargo not installed
        return None, f"could not run cargo: {error}"
    if completed.returncode != 0:
        tail = (completed.stderr or "").strip().splitlines()[-3:]
        return None, "cargo tree -e features failed: " + " | ".join(tail)
    return completed.stdout, None


def ignored_rustsec_ids():
    """The `advisories.ignore` list of every configured dependency gate."""
    lists = {}
    for path in AUDIT_CONFIGS:
        if not path.exists():
            lists[path] = None
            continue
        try:
            document = tomllib.loads(path.read_text())
        except tomllib.TOMLDecodeError as error:
            lists[path] = None
            print(
                "check-advisory-scope: %s is not valid TOML: %s"
                % (path.relative_to(ROOT), error),
                file=sys.stderr,
            )
            continue
        lists[path] = set(document.get("advisories", {}).get("ignore", []))
    return lists


# Advisories cargo-audit sees but cargo-deny never does.
#
# cargo-audit audits every package listed in Cargo.lock. cargo-deny builds a graph for
# the four targets in deny.toml's `[graph]` and reports `advisory-not-detected` for an
# ignore entry whose package is not in that graph. For a package that is in the lock but
# in no configured target's graph -- a stale lock entry -- the two disagree by
# construction: the ignore belongs in `.cargo/audit.toml` and must NOT be in deny.toml.
#
# Naming them here keeps that divergence from hiding a real gap. The split is checked in
# both directions, so an entry cannot be left behind once the package leaves the lock,
# and cannot be added without being named here with a reason.
LOCK_ONLY_ADVISORIES = {
    "RUSTSEC-2023-0071": (
        "rsa 0.9.10, reached only through sqlx-mysql, which no crate in this workspace "
        "enables (stale lock entry). No target in deny.toml's [graph] builds it."
    ),
}


def check_ignore_list_split(audit_ids, deny_ids):
    """The two ignore lists are allowed to differ only where LOCK_ONLY_ADVISORIES says."""
    failures = []
    only_deny = sorted(deny_ids - audit_ids)
    if only_deny:
        failures.append(
            "deny.toml ignores %s, which .cargo/audit.toml does not; cargo-deny must not "
            "suppress something cargo-audit is unaware of" % ", ".join(only_deny)
        )
    only_audit = audit_ids - deny_ids
    undeclared = sorted(only_audit - set(LOCK_ONLY_ADVISORIES))
    if undeclared:
        failures.append(
            "ignored in .cargo/audit.toml but absent from deny.toml and not declared "
            "lock-only: %s; either add it to deny.toml or name it in "
            "LOCK_ONLY_ADVISORIES with the reason it is unreachable there"
            % ", ".join(undeclared)
        )
    obsolete = sorted(set(LOCK_ONLY_ADVISORIES) - only_audit)
    if obsolete:
        failures.append(
            "LOCK_ONLY_ADVISORIES names %s, but that is no longer the split; remove the "
            "entry (the advisory is either matched by cargo-deny now, or no longer "
            "ignored at all)" % ", ".join(obsolete)
        )
    return failures


def load_records():
    if not RECORDS.exists():
        sys.exit("check-advisory-scope: missing %s" % RECORDS.relative_to(ROOT))
    with RECORDS.open("rb") as handle:
        document = tomllib.load(handle)
    records = document.get("advisory", [])
    if not records:
        sys.exit("check-advisory-scope: security/advisory-scope.toml declares no [[advisory]] records")
    problems = []
    for record in records:
        where = record.get("id", "<record with no id>")
        status = record.get("status", "not_affected")
        if status not in STATUSES:
            problems.append("%s: unknown status %r" % (where, status))
        for field in ("package", "published_range", "vulnerable_range", "evidence"):
            if not record.get(field):
                problems.append("%s: missing %s" % (where, field))
        if status == "unreachable" and not record.get("requires_absent_features"):
            problems.append(
                "%s: status 'unreachable' needs requires_absent_features naming the "
                "precondition the advisory states" % where
            )
        if status == "accepted_risk" and not record.get("accepted_reason"):
            problems.append(
                "%s: status 'accepted_risk' needs accepted_reason" % where
            )
    if problems:
        for problem in problems:
            print("check-advisory-scope: FAIL: " + problem, file=sys.stderr)
        sys.exit(1)
    return records


def main():
    parser = argparse.ArgumentParser(description="Check resolved versions against verified advisory windows.")
    parser.add_argument("--list", action="store_true", help="print the records and exit")
    args = parser.parse_args()

    records = load_records()
    if args.list:
        for record in records:
            print(
                "%s  %-14s %-13s published=%-22s vulnerable=%-22s resolved=%s" % (
                    record["id"],
                    record["package"],
                    record.get("status", "not_affected"),
                    record["published_range"],
                    record["vulnerable_range"],
                    sorted(record.get("expected_resolved", []), key=parse_version),
                )
            )
        return 0

    locks = lockfiles()
    if not locks:
        print("check-advisory-scope: no Cargo.lock found in the tree", file=sys.stderr)
        return 1

    resolved = {}
    for path in locks:
        for name, version in parse_lock(path.read_text()):
            resolved.setdefault(name, {}).setdefault(version, []).append(str(path.relative_to(ROOT)))

    failures = []
    feature_tree = None
    feature_error = None
    ignore_lists = ignored_rustsec_ids()

    audit_ids = ignore_lists.get(AUDIT_CONFIGS[0])
    deny_ids = ignore_lists.get(AUDIT_CONFIGS[1])
    if audit_ids is None or deny_ids is None:
        failures.append(
            "the ignore lists could not be read from both %s and %s"
            % (AUDIT_CONFIGS[0].relative_to(ROOT), AUDIT_CONFIGS[1].relative_to(ROOT))
        )
    else:
        failures.extend(check_ignore_list_split(audit_ids, deny_ids))


    for record in records:
        package = record["package"]
        status = record.get("status", "not_affected")
        expected = record.get("expected_resolved")
        versions = sorted(resolved.get(package, {}), key=parse_version)
        in_range = [v for v in versions if satisfies(parse_version(v), record["vulnerable_range"])]

        if record.get("rustsec"):
            for path, ignored in ignore_lists.items():
                if ignored is None:
                    failures.append(
                        "%s: %s could not be read, so its ignore list cannot be checked"
                        % (record["id"], path.relative_to(ROOT))
                    )
                elif record["rustsec"] not in ignored:
                    failures.append(
                        "%s: %s is not in the ignore list of %s; the two dependency "
                        "gates disagree about this advisory"
                        % (record["id"], record["rustsec"], path.relative_to(ROOT))
                    )

        evidence_path = ROOT / record["evidence"]
        if not evidence_path.exists():
            failures.append(
                "%s: the record cites %s, which does not exist - the justification is "
                "missing" % (record["id"], record["evidence"])
            )

        if expected is not None:
            if versions != sorted(set(expected), key=parse_version):
                failures.append(
                    "%s: %s resolves to %s but the record expects %s; re-verify the window "
                    "and update %s (evidence: %s)" % (
                        record["id"], package, versions or ["<absent>"],
                        sorted(set(expected), key=parse_version),
                        RECORDS.relative_to(ROOT), record["evidence"],
                    )
                )

        if any("-" in v for v in versions) and any(
            "-" in spec for spec in (record["published_range"], record["vulnerable_range"])
        ):
            failures.append(
                "%s: %s resolves to a pre-release (%s) and the record's ranges use a "
                "pre-release bound; this check compares numeric parts only and will not "
                "guess at semver precedence here" % (
                    record["id"], package, ", ".join(v for v in versions if "-" in v),
                )
            )

        in_published = [
            v for v in versions if satisfies(parse_version(v), record["published_range"])
        ]

        if status == "not_affected":
            for version in in_range:
                where = ", ".join(resolved[package][version])
                failures.append(
                    "%s: resolved %s %s (%s) is inside the vulnerable window %s - see %s" % (
                        record["id"], package, version, where,
                        record["vulnerable_range"], record["evidence"],
                    )
                )
        elif status == "accepted_risk":
            if not in_range:
                failures.append(
                    "%s: no resolved %s version is inside %s any more, so the accepted risk "
                    "no longer describes a real exposure; drop the record and the ignore "
                    "entries it justifies, and update %s" % (
                        record["id"], package, record["vulnerable_range"], record["evidence"],
                    )
                )
        elif status == "unreachable":
            if feature_tree is None and feature_error is None:
                feature_tree, feature_error = enabled_feature_tree()
            if feature_error:
                failures.append(
                    "%s: the record claims a feature precondition, but it could not be "
                    "checked: %s" % (record["id"], feature_error)
                )
            else:
                present = present_features(feature_tree, record["requires_absent_features"])
                if present:
                    failures.append(
                        "%s: %s is enabled in this graph, which is the precondition the "
                        "advisory names - the vulnerable code is compiled now; see %s"
                        % (record["id"], ", ".join(present), record["evidence"])
                    )

        if not in_published:
            print(
                "check-advisory-scope: note - no resolved %s version is inside the published range "
                "%s any more; the record is kept as the reason the alert could not be acted on, "
                "but it may now be obsolete." % (package, record["published_range"]),
                file=sys.stderr,
            )

    if failures:
        for failure in failures:
            print("check-advisory-scope: FAIL: %s" % failure, file=sys.stderr)
        return 1

    print(
        "check-advisory-scope: OK - %d advisory record(s) verified against %d lockfile(s)" % (
            len(records), len(locks),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
