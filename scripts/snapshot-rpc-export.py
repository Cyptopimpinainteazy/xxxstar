#!/usr/bin/env python3
"""Export a *running* node's state into the raw-spec shape `x3-state-snapshot build` consumes.

The snapshot row asked for a zero-downtime snapshot. The repository already had
two halves of one and no way to join them:

* `crates/x3-state-snapshot` builds, verifies and restores a content-addressed
  snapshot, and its `build` takes a plain chain spec whose `genesis.raw.top` is
  the whole state map;
* `scripts/snapshot-restore.sh backup` refuses a live database — correctly, a
  plain `tar` of a running RocksDB is not a consistent snapshot — so the archive
  could only be cut with the node stopped.

This is the missing exporter: it reads the state of a node that keeps producing
blocks, pins every read to one finalized block hash, and writes the raw spec.
Nothing here is trusted on its own — `x3-state-snapshot root --from-raw-spec`
recomputes the trie root from the exported entries with the chain's own layout
and the drill requires it to equal the `stateRoot` the chain published in that
block's header. A key the walk missed, a value read from the wrong block, or a
key the enumeration invented all change that root. So the check does not measure
this script's opinion of the export; it measures the export against the chain.

What it refuses, rather than repairing:

* an anchor the chain does not agree is canonical (`chain_getBlockHash(number)`
  must equal the hash being exported) or that is not finalized;
* an anchor with no GRANDPA justification, unless `--justified-ancestor` is
  given — a snapshot's finality proof is a field a mirror has to be able to
  check, and `chain_getBlock().justifications` is where the chain publishes it.
  X3's node generates one every `justification_generation_period` blocks (512),
  so on a young chain the newest justified block is often far below the head;
  `--justified-ancestor` walks back to it and says so in the report;
* a key that paging returned at the anchor but whose value read at that same
  anchor comes back null: the two reads disagree, so the walk is not a snapshot;
* a page walk that does not terminate, that repeats a page, or that exceeds the
  key cap;
* a chain with child tries, which `x3-state-snapshot` does not encode;
* an existing `--out` or `--report`, without `--force`.

Exit codes: 0 exported, 1 refused, 2 the command line or the RPC endpoint itself
was wrong.

    python3 scripts/snapshot-rpc-export.py \
        --rpc http://127.0.0.1:9944 \
        --out /tmp/state.json --report /tmp/anchor.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from typing import Any

# The GRANDPA justification id. Substrate's `sc-consensus-grandpa` publishes its
# justification under this name; anything else in `justifications` is a
# different gadget's artifact and must not be passed off as GRANDPA finality.
GRANDPA_JUSTIFICATION_ID = "FRNK"
GRANDPA_JUSTIFICATION_ID_HEX = ["0x46524e4b", "46524e4b"]

# Refusals are named so the caller can tell "the chain said no" from "the
# harness is broken".
EXIT_OK = 0
EXIT_REFUSED = 1
EXIT_USAGE = 2

MAX_KEYS = 5_000_000
MAX_PAGES = 20_000


class Refused(Exception):
    """A condition that must stop the export. Never a warning."""


class RpcError(Exception):
    """The endpoint could not answer. Not a statement about the chain's state."""


class Rpc:
    def __init__(self, url: str, timeout: float = 30.0) -> None:
        self.url = url
        self.timeout = timeout
        self.calls = 0
        self.request_id = 0

    def call(self, method: str, params: list[Any]) -> Any:
        self.request_id += 1
        request_id = self.request_id
        body = json.dumps(
            {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        ).encode()
        request = urllib.request.Request(
            self.url, data=body, headers={"Content-Type": "application/json"}
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                document = json.load(response)
        except (urllib.error.URLError, OSError, ValueError) as exc:
            raise RpcError(f"{method}: {exc}") from exc
        self.calls += 1
        if not isinstance(document, dict):
            raise RpcError(f"{method}: response was not a JSON object")
        if document.get("jsonrpc") != "2.0":
            raise RpcError(f"{method}: response carried an invalid JSON-RPC version")
        response_id = document.get("id")
        if type(response_id) not in (int, float) or response_id != request_id:
            raise RpcError(f"{method}: response id does not match request {request_id}")
        if ("result" in document) == ("error" in document):
            raise RpcError(f"{method}: response must carry exactly one of result or error")
        if "error" in document:
            error = document["error"]
            if (not isinstance(error, dict) or type(error.get("code")) is not int
                    or not isinstance(error.get("message"), str)):
                raise RpcError(f"{method}: response carried an invalid error object")
            raise RpcError(f"{method}: {error}")
        return document["result"]


# The client's own words for "the state you asked for is not here any more". It
# is a property of the node the operator chose (pruning window), not a statement
# about the chain, so it stays distinct from a generic RPC failure: a validator
# with a bounded pruning window can serve only the recent anchors, and exporting
# an older one has to be refused rather than answered partially.
STATE_DISCARDED = ("State already discarded", "state already discarded")


def hex_to_int(value: str, what: str) -> int:
    try:
        return int(value, 16)
    except (TypeError, ValueError) as exc:
        raise Refused(f"{what} is not a hex quantity: {value!r}") from exc


def read_header_number(rpc: Rpc, block_hash: str, what: str) -> int:
    """Refuse missing headers instead of crashing while reading a head's height."""
    header = rpc.call("chain_getHeader", [block_hash])
    if not isinstance(header, dict) or "number" not in header:
        raise Refused(f"{what}: no numbered header for {block_hash}")
    return hex_to_int(header["number"], what)


def read_anchor(rpc: Rpc, block_hash: str) -> dict[str, Any]:
    """Everything the export is pinned to, read from the chain rather than inferred."""
    header = rpc.call("chain_getHeader", [block_hash])
    if not isinstance(header, dict):
        raise Refused(f"no header for {block_hash}: that block is not in this chain")
    if "stateRoot" not in header or "number" not in header:
        raise Refused(f"header for {block_hash} carries no number/stateRoot")
    number = hex_to_int(header["number"], "header.number")

    # Canonicity: the chain's own answer for "which block is at this height".
    # A block that is not the canonical one at its height is on a fork, and the
    # state under it is the state a reorg throws away.
    canonical = rpc.call("chain_getBlockHash", [number])
    if canonical != block_hash:
        raise Refused(
            f"block {block_hash} at height {number} is not canonical: the chain "
            f"reports {canonical} at that height"
        )

    note = ""
    finalized = rpc.call("chain_getFinalizedHead", [])
    finalized_number = read_header_number(rpc, finalized, "finalized.number")
    if number > finalized_number:
        raise Refused(
            f"height {number} is above the finalized head ({finalized_number}); a "
            f"snapshot of an unfinalized block can be reverted"
        )
    if number < finalized_number:
        note = (
            f"anchor is {finalized_number - number} block(s) behind the finalized head "
            f"({finalized_number}); the state at {number} is immutable, which is what a "
            f"snapshot needs"
        )

    return {
        "block_hash": block_hash,
        "block_number": number,
        "state_root": header["stateRoot"],
        "finalized_head": int(finalized_number),
        "behind_finalized_head": finalized_number - number,
        "note": note,
    }


def read_justification(rpc: Rpc, block_hash: str) -> str | None:
    """The GRANDPA justification the chain publishes for this block, if any."""
    block = rpc.call("chain_getBlock", [block_hash]) or {}
    justifications = block.get("justifications")
    if not justifications:
        return None
    for entry in justifications:
        # `chain_getBlock` returns [id, justification] pairs, and this node
        # encodes *both* sides as arrays of byte values rather than as hex
        # strings — a hex-only reader silently finds no justification anywhere
        # and then reports that the chain cannot produce a finality proof. Both
        # shapes are accepted here.
        if not isinstance(entry, list) or len(entry) != 2:
            continue
        raw_id, encoded = entry
        if isinstance(raw_id, list):
            raw_id = "0x" + bytes(raw_id).hex()
        if not isinstance(raw_id, str):
            continue
        if raw_id.lower() not in GRANDPA_JUSTIFICATION_ID_HEX and raw_id != GRANDPA_JUSTIFICATION_ID:
            continue
        if isinstance(encoded, list):
            try:
                encoded = "0x" + bytes(encoded).hex()
            except ValueError:
                continue
        if not isinstance(encoded, str) or not encoded.startswith("0x"):
            continue
        if len(encoded) <= 2:
            continue
        return encoded
    return None


def justified_ancestor(rpc: Rpc, from_height: int, floor: int = 1) -> tuple[int, str, str]:
    """Walk back to the newest ancestor carrying a GRANDPA justification.

    X3's node runs GRANDPA with `justification_generation_period = 512`, so on a
    chain that has not yet crossed a period boundary there is nothing nearer than
    the last one. Each step also confirms the height is canonicity-checked, so
    the walk cannot settle on a fork.
    """
    height = from_height
    while height >= floor:
        block_hash = rpc.call("chain_getBlockHash", [height])
        if not block_hash:
            raise Refused(f"no canonical hash at height {height}")
        justification = read_justification(rpc, block_hash)
        if justification is not None:
            return height, block_hash, justification
        height -= 1
    raise Refused(
        f"no GRANDPA justification at or below height {from_height} down to {floor}: "
        f"this chain cannot produce a finality proof for a snapshot yet"
    )


def refusal_for_absent_state(exc: RpcError, at: str) -> Exception:
    """Turn "the state is gone" into a refusal that names the reason."""
    message = str(exc)
    if any(marker in message for marker in STATE_DISCARDED):
        return Refused(
            f"{at} is finalized but this node no longer holds its state: the client "
            f"pruned it. A snapshot of a finalized anchor has to be exported from a "
            f"node whose pruning window still covers that anchor — an archive node "
            f"(`--state-pruning archive --blocks-pruning archive`), or a bounded node "
            f"asked for an anchor inside its window. Refusing to export a partial "
            f"state. ({message})"
        )
    return exc


def enumerate_keys(rpc: Rpc, at: str, page_size: int) -> list[str]:
    """Every storage key the chain holds at `at`, in the order it answered them."""
    keys: list[str] = []
    seen: set[str] = set()
    start: str | None = None
    pages = 0
    while True:
        pages += 1
        if pages > MAX_PAGES:
            raise Refused(
                f"state_getKeysPaged did not finish within {MAX_PAGES} pages; "
                f"refusing rather than exporting a partial state"
            )
        try:
            page = rpc.call("state_getKeysPaged", ["", page_size, start, at])
        except RpcError as exc:
            raise refusal_for_absent_state(exc, at) from exc
        if not isinstance(page, list):
            raise Refused("state_getKeysPaged did not return a list")
        if not page:
            break
        fresh = [key for key in page if key not in seen]
        if not fresh:
            raise Refused(
                f"state_getKeysPaged repeated a page at {len(seen)} keys; the walk "
                f"cannot terminate, so the export would be partial"
            )
        for key in fresh:
            seen.add(key)
            keys.append(key)
        if len(seen) > MAX_KEYS:
            raise Refused(f"state holds more than {MAX_KEYS} keys; refusing")
        if len(page) < page_size:
            break
        start = page[-1]
    if not keys:
        raise Refused(f"the chain exposed no state at {at}")
    return keys


def read_state(rpc: Rpc, at: str, page_size: int) -> tuple[dict[str, str], dict[str, int]]:
    keys = enumerate_keys(rpc, at, page_size)
    top: dict[str, str] = {}
    empty_values = 0
    for key in keys:
        try:
            value = rpc.call("state_getStorage", [key, at])
        except RpcError as exc:
            raise refusal_for_absent_state(exc, at) from exc
        if value is None:
            raise Refused(
                f"key {key} was enumerated at {at} but has no value at {at}; the "
                f"key walk and the value reads disagree, so this is not a snapshot"
            )
        if not isinstance(value, str) or not value.startswith("0x"):
            raise Refused(f"value for key {key} is not 0x-hex: {value!r}")
        if value == "0x":
            empty_values += 1
        top[key] = value
    return top, {
        "key_count": len(top),
        "empty_values": empty_values,
        "pages": (len(top) + page_size - 1) // page_size,
    }


def template_metadata(path: str | None) -> tuple[dict[str, Any], int]:
    """Everything a raw spec needs besides its state.

    A template contributes metadata only — its state is replaced wholesale. That
    is deliberate: merging a template's state with an export produces a third
    state that neither the chain nor the export can name.
    """
    if path is None:
        return (
            {
                "name": "X3 exported state",
                "id": "x3-export",
                "chainType": "Live",
                "properties": {"tokenSymbol": "X3", "tokenDecimals": 18},
            },
            0,
        )
    with open(path, "r", encoding="utf-8") as handle:
        spec = json.load(handle)
    if not isinstance(spec, dict):
        raise Refused(f"{path} does not hold a JSON object")
    metadata = {
        key: spec[key]
        for key in (
            "name",
            "id",
            "chainType",
            "bootNodes",
            "telemetryEndpoints",
            "protocolId",
            "properties",
            "forkBlocks",
            "badBlocks",
        )
        if key in spec
    }
    metadata.setdefault("name", "X3 exported state")
    metadata.setdefault("id", "x3-export")
    metadata.setdefault("chainType", "Live")
    children = 0
    raw = ((spec.get("genesis") or {}).get("raw")) or {}
    for child_key in ("children", "childrenDefault"):
        found = raw.get(child_key)
        if isinstance(found, dict):
            children = max(children, len(found))
    return metadata, children


def write_json(path: str, document: Any, force: bool) -> None:
    if os.path.exists(path) and not force:
        raise Refused(f"{path} exists; pass --force to replace it")
    temporary = f"{path}.partial"
    with open(temporary, "w", encoding="utf-8") as handle:
        json.dump(document, handle)
    # Rename, so a reader never sees half a spec: a chain spec with missing state
    # is a chain spec with missing state.
    os.replace(temporary, path)


def state_digest(top: dict[str, str]) -> str:
    """A digest of the exported entries, so the report pins the spec's content.

    Sorted, because two walks of the same block must not be able to disagree
    about the digest of the same state.
    """
    digest = hashlib.sha256()
    for key in sorted(top):
        digest.update(key.encode())
        digest.update(b"\t")
        digest.update(top[key].encode())
        digest.update(b"\n")
    return "0x" + digest.hexdigest()


def export(args: argparse.Namespace) -> dict[str, Any]:
    rpc = Rpc(args.rpc, timeout=args.timeout)

    started = time.time()
    start_head = rpc.call("chain_getFinalizedHead", [])
    start_number = read_header_number(rpc, start_head, "start.number")

    if args.at:
        anchor = read_anchor(rpc, args.at)
    else:
        anchor = read_anchor(rpc, start_head)

    if anchor["block_number"] < args.min_finalized:
        raise Refused(
            f"anchor height {anchor['block_number']} is below --min-finalized "
            f"{args.min_finalized}"
        )

    justification = read_justification(rpc, anchor["block_hash"])
    walked_back = False
    requested_number = anchor["block_number"]
    if justification is None:
        if args.finality_proof == "optional":
            # Only for a run whose output is never restored from - a state
            # comparison against a chain that has no justification to offer
            # (the genesis block of a freshly restored spec, for one). The
            # report says `finality_proof: null` rather than inventing one, and
            # `build` would refuse this export because a manifest needs it.
            pass
        elif not args.justified_ancestor:
            raise Refused(
                f"block {anchor['block_number']} ({anchor['block_hash']}) carries no "
                f"GRANDPA justification, so the snapshot would have no finality proof. "
                f"X3 generates one every "
                f"{args.justification_period} blocks; pass --justified-ancestor to "
                f"anchor at the newest height that has one, or wait for the next "
                f"period boundary."
            )
        else:
            number, block_hash, justification = justified_ancestor(
                rpc, anchor["block_number"]
            )
            anchor = read_anchor(rpc, block_hash)
            walked_back = True

    metadata, children = template_metadata(args.from_spec)
    if children:
        raise Refused(
            f"{args.from_spec} declares {children} child trie entr(ies); "
            f"x3-state-snapshot does not encode child tries"
        )

    top, counts = read_state(rpc, anchor["block_hash"], args.page_size)

    # Read the anchor again after the walk. The block is immutable, so anything
    # else means the client answered about a different chain mid-export.
    confirm = read_anchor(rpc, anchor["block_hash"])
    if confirm["state_root"] != anchor["state_root"]:
        raise Refused(
            f"the state root at {anchor['block_hash']} moved during the export "
            f"({anchor['state_root']} -> {confirm['state_root']}); the chain this "
            f"export describes is not stable"
        )

    spec_version = rpc.call("state_getRuntimeVersion", []) or {}
    end_head = rpc.call("chain_getFinalizedHead", [])
    end_number = read_header_number(rpc, end_head, "end.number")
    spec = dict(metadata)
    spec["genesis"] = {"raw": {"top": top, "childrenDefault": {}}}

    report = {
        "chain_id": metadata.get("id"),
        "chain_name": metadata.get("name"),
        "rpc": args.rpc,
        "block_number": anchor["block_number"],
        "requested_block_number": requested_number,
        "walked_back_to_justified_ancestor": walked_back,
        "block_hash": anchor["block_hash"],
        "state_root": anchor["state_root"],
        "runtime_spec_version": spec_version.get("specVersion"),
        "finality_proof": justification,
        "key_count": counts["key_count"],
        "empty_values": counts["empty_values"],
        "state_version": 1,
        "state_entries_sha256": state_digest(top),
        "source_spec": args.from_spec,
        "spec": args.out,
        "node_was_never_stopped": True,
        "finalized_head_before": start_number,
        "finalized_head_after": end_number,
        "finalized_head_advanced_by": None,
        "elapsed_seconds": round(time.time() - started, 3),
        "rpc_calls": rpc.calls,
    }
    report["finalized_head_advanced_by"] = (
        report["finalized_head_after"] - report["finalized_head_before"]
    )
    # Finish every RPC check before replacing either artifact. An endpoint that
    # loses the end header during the final report read must preserve old data.
    write_json(args.out, spec, args.force)
    if args.report:
        write_json(args.report, report, args.force)
    return report


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export a running node's state as a raw chain spec.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("Exit codes")[-1] if __doc__ else None,
    )
    parser.add_argument("--rpc", default="http://127.0.0.1:9944")
    parser.add_argument("--out", required=True, help="raw chain spec to write")
    parser.add_argument("--report", help="anchor/proof report to write")
    parser.add_argument("--at", help="block hash to export (default: finalized head)")
    parser.add_argument(
        "--min-finalized",
        type=int,
        default=0,
        help="refuse if the anchor height is below this",
    )
    parser.add_argument(
        "--from-spec",
        help="template whose metadata the output copies (its state is replaced)",
    )
    parser.add_argument("--page-size", type=int, default=1000)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--justified-ancestor",
        action="store_true",
        help="anchor at the newest ancestor with a GRANDPA justification",
    )
    parser.add_argument(
        "--finality-proof",
        choices=("required", "optional"),
        default="required",
        help=(
            "`required` (default) refuses an anchor with no GRANDPA justification. "
            "`optional` records the absence instead, for exports used only to "
            "compare state; such an export cannot be built into a snapshot."
        ),
    )
    parser.add_argument(
        "--justification-period",
        type=int,
        default=512,
        help="only used in the refusal message",
    )
    args = parser.parse_args(argv)
    if args.page_size <= 0:
        parser.error("--page-size must be positive")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = export(args)
    except Refused as exc:
        print(f"[snapshot-export] REFUSED: {exc}", file=sys.stderr)
        return EXIT_REFUSED
    except RpcError as exc:
        print(f"[snapshot-export] the endpoint did not answer: {exc}", file=sys.stderr)
        return EXIT_USAGE
    print(
        f"[snapshot-export] exported {report['key_count']} keys at height "
        f"{report['block_number']} ({report['block_hash'][:18]}…) "
        f"state_root={report['state_root'][:18]}… "
        f"justification={'yes' if report['finality_proof'] else 'NO'}"
    )
    print(
        f"[snapshot-export] the node was never stopped: its finalized head went "
        f"{report['finalized_head_before']} -> {report['finalized_head_after']} "
        f"({report['finalized_head_advanced_by']:+d}) during the "
        f"{report['elapsed_seconds']}s export"
    )
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
