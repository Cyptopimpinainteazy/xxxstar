#!/usr/bin/env python3
"""Hand an x3 failure packet to the router and store the root-cause answer.

The packet is the evidence: a reproducibility record produced by a real run
(or by a gate wrapper for a failing cargo test). This script only transports
it and validates the answer. It never invents a cause, never edits code, and
never guesses when the router is down: a missing router is exit 2 with the
packet untouched, not a fabricated diagnosis.

Exit codes (fail-closed, matching the rest of the tooling):
    0  a contract-valid root-cause answer was stored
    2  the packet is unreadable, or nothing answered at the router address
    3  something answered (even an HTTP error), but not with the required
       JSON contract; the raw answer is preserved for inspection
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

SUPPORTED_SCHEMAS = (
    "x3-failure-packet-v2",
    "x3-failure-packet-v1",
    "x3-gate-failure-packet-v1",
)
DEFAULT_ROUTER = "http://127.0.0.1:11435"
DEFAULT_MODEL = "x3-auto"
STATE_EXCERPT_CHARS = 4000

SYSTEM_PROMPT = """You are the X3 root-cause agent. You receive a failure packet from a
deterministic X3 simulator or gate. Rank the code locations most likely
responsible, top first. Answer in JSON only, with this exact shape:
{"causes":[{"symbol":"crate::Type::function","file":"path/to/file.rs",
"confidence":0.0,"reasoning":"why this location, from the packet"}],
"first_experiment":"one command or check that would confirm or refute the top cause",
"required_regression_test":"the test that must exist after the fix",
"notes":"assumptions and gaps"}. Confidence is your probability estimate, not a
fact. If the packet does not identify a cause, say so in notes and return an
empty causes list rather than guessing."""


# Fields without which a packet cannot identify a failure. A file that names
# a schema but omits these is not evidence, and dispatching it would let a
# model answer about a failure nobody can see.
SIM_REQUIRED_FIELDS = (
    "failure_id",
    "producer",
    "scenario",
    "invariant",
    "session",
    "detail",
    "replay_command",
)
GATE_REQUIRED_FIELDS = (
    "failure_id",
    "gate",
    "command",
    "first_error",
    "replay_command",
)


def _require_text_fields(packet: dict, path: Path, names) -> None:
    missing = [
        name
        for name in names
        if not isinstance(packet.get(name), str) or not packet[name].strip()
    ]
    if missing:
        raise SystemExit(
            f"root_cause: {path} is not a usable failure packet — "
            f"missing non-empty field(s): {', '.join(missing)}"
        )


def load_packet(path: Path) -> dict:
    try:
        packet = json.loads(path.read_text())
    except (OSError, ValueError) as exc:
        raise SystemExit(f"root_cause: cannot read packet {path}: {exc}") from exc
    if not isinstance(packet, dict) or packet.get("schema") not in SUPPORTED_SCHEMAS:
        raise SystemExit(
            f"root_cause: {path} is not a supported failure packet "
            f"(schema must be one of {SUPPORTED_SCHEMAS})"
        )
    if packet["schema"].startswith("x3-failure-packet-"):
        _require_text_fields(packet, path, SIM_REQUIRED_FIELDS)
        if isinstance(packet.get("seed"), bool) or not isinstance(packet.get("seed"), int):
            raise SystemExit(f"root_cause: {path} has no numeric 'seed'")
        config = packet.get("config")
        if not isinstance(config, dict) or any(
            isinstance(config.get(key), bool) or not isinstance(config.get(key), int)
            for key in ("sessions", "steps", "nodes")
        ):
            raise SystemExit(
                f"root_cause: {path} has no numeric config (sessions/steps/nodes)"
            )
    else:
        _require_text_fields(packet, path, GATE_REQUIRED_FIELDS)
        if isinstance(packet.get("exit_code"), bool) or not isinstance(
            packet.get("exit_code"), int
        ):
            raise SystemExit(f"root_cause: {path} has no numeric 'exit_code'")
    return packet


def _truncate(value, limit=STATE_EXCERPT_CHARS):
    text = value if isinstance(value, str) else json.dumps(value, indent=2, default=str)
    if len(text) <= limit:
        return text
    return text[:limit] + f"\n... [{len(text) - limit} characters omitted]"


def _safe_component(value: str) -> str:
    """One filename component: no separators, no traversal, never empty."""
    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "-", value).strip("-")
    return cleaned or "unknown"


def _list_field(packet: dict, name: str) -> list:
    """A renderable optional list: anything else is not evidence to prompt on."""
    value = packet.get(name)
    return value if isinstance(value, list) else []


def build_prompt(packet: dict) -> str:
    """Render the packet as the facts an investigator needs, in order."""
    lines = [
        f"failure_id: {packet.get('failure_id', 'unknown')}",
        f"producer: {packet.get('producer', packet.get('gate', 'unknown'))}",
        f"commit: {packet.get('commit', 'unknown')}",
        f"invariant: {packet.get('invariant', packet.get('first_error', 'unknown'))}",
        f"session: {packet.get('session', 'unknown')}",
        f"detail: {packet.get('detail', packet.get('message', ''))}",
    ]
    if packet.get("command"):
        lines.append(f"gate command: {packet['command']}")
    if packet.get("exit_code") is not None:
        lines.append(f"gate exit code: {packet['exit_code']}")
    for test in _list_field(packet, "failing_tests"):
        lines.append(f"failing test: {test}")
    if packet.get("seed") is not None:
        lines.append(
            "run: seed={seed} scenario={scenario} sessions={sessions} steps={steps} nodes={nodes}".format(
                seed=packet.get("seed"),
                scenario=packet.get("scenario"),
                sessions=(packet.get("config") or {}).get("sessions"),
                steps=(packet.get("config") or {}).get("steps"),
                nodes=(packet.get("config") or {}).get("nodes"),
            )
        )
    if packet.get("first_bad_step_label"):
        lines.append(
            "first bad step: {} op={}".format(
                packet.get("first_bad_step_label"), packet.get("first_bad_op")
            )
        )
    for fault in _list_field(packet, "active_faults"):
        lines.append(f"active fault: {fault}")
    suspects = _list_field(packet, "suspected_code")
    if suspects:
        lines.append("suspected code (pointers, not verdicts):")
        for suspect in suspects:
            if isinstance(suspect, dict):
                lines.append(
                    "  - {symbol} in {file} — {reason}".format(
                        symbol=suspect.get("symbol", "?"),
                        file=suspect.get("file", "?"),
                        reason=suspect.get("reason", ""),
                    )
                )
    if packet.get("suspected_file"):
        lines.append(f"suspected file: {packet['suspected_file']}")
    violations = _list_field(packet, "all_violations")
    if violations:
        lines.append("all violations in this run:")
        for violation in violations:
            if not isinstance(violation, dict):
                continue
            lines.append(
                "  - {} session={}: {}".format(
                    violation.get("code", "?"),
                    violation.get("session", "?"),
                    violation.get("detail", ""),
                )
            )
    if packet.get("state_before") is not None:
        lines.append("state before the bad step:\n" + _truncate(packet["state_before"]))
    if packet.get("state_after") is not None:
        lines.append("state after the bad step:\n" + _truncate(packet["state_after"]))
    if packet.get("suspected_lines"):
        lines.append("suspected lines: " + ", ".join(map(str, packet["suspected_lines"])))
    for location in _list_field(packet, "locations"):
        if isinstance(location, dict) and location.get("file"):
            lines.append(
                "location: {file}:{line}".format(
                    file=location["file"], line=location.get("line", "?")
                )
            )
    if packet.get("log_excerpt"):
        lines.append("log excerpt:\n" + _truncate(packet["log_excerpt"]))
    if packet.get("required_regression_test"):
        lines.append("required regression test: " + str(packet["required_regression_test"]))
    if packet.get("replay_command"):
        lines.append(f"replay: {packet['replay_command']}")
    if packet.get("minimized"):
        minimized = packet["minimized"] or {}
        if minimized.get("verified"):
            lines.append(f"minimized (verified): {minimized.get('replay_command')}")
    if packet.get("command"):
        lines.append(f"failing command: {packet['command']} (exit {packet.get('exit_code')})")
    return "\n".join(lines)


def request_payload(packet: dict, model: str, max_tokens: int) -> dict:
    return {
        "model": model,
        "temperature": 0,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {
                "role": "user",
                "content": "Failure packet:\n\n" + build_prompt(packet)
                + "\n\nRank the responsible code locations.",
            },
        ],
    }


def extract_contract(content: str) -> dict:
    """Parse the model answer as the required JSON contract, or raise."""
    text = content.strip()
    if text.startswith("```"):
        lines = text.splitlines()
        text = "\n".join(lines[1:-1] if lines[-1].strip().startswith("```") else lines[1:])
    try:
        answer = json.loads(text)
    except ValueError as exc:
        raise ValueError(f"answer is not JSON: {exc}") from exc
    if not isinstance(answer, dict) or not isinstance(answer.get("causes"), list):
        raise ValueError("answer has no 'causes' list")
    for cause in answer["causes"]:
        if not isinstance(cause, dict) or not cause.get("symbol"):
            raise ValueError("every cause needs a 'symbol'")
        if not isinstance(cause.get("file"), str) or not cause.get("file").strip():
            raise ValueError("every cause needs a 'file'; a symbol without a file is not actionable")
        confidence = cause.get("confidence")
        # `bool` is an `int` in Python; `true` is not a confidence.
        if isinstance(confidence, bool) or not isinstance(confidence, (int, float)):
            raise ValueError("every cause needs a numeric 'confidence'")
        if not 0.0 <= float(confidence) <= 1.0:
            raise ValueError("'confidence' must be between 0.0 and 1.0")
        reasoning = cause.get("reasoning")
        if not isinstance(reasoning, str) or not reasoning.strip():
            raise ValueError("every cause needs non-empty 'reasoning'")
    if answer.get("causes"):
        for field in ("first_experiment", "required_regression_test"):
            value = answer.get(field)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"a ranked answer needs a non-empty '{field}'")
    notes = answer.get("notes")
    if not isinstance(notes, str) or not notes.strip():
        # A negative answer has to say *why* it found nothing; a ranked one
        # has to state its assumptions. Either way, silence is not a cause.
        raise ValueError("the answer needs non-empty 'notes'")
    return answer


def store(packet: dict, answer: dict, response: dict, model: str, out_dir: Path) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    kind = packet.get("invariant") or packet.get("gate") or "failure"
    # A gate may be named `crates/foo`: the kind and the id are pinned into a
    # file name, so separators become dashes instead of subdirectories.
    stem = "{}-{}".format(
        _safe_component(str(packet.get("failure_id", "unknown"))),
        _safe_component(str(kind).lower()),
    )
    record = {
        "schema": "x3-root-cause-v1",
        "stored_at": datetime.now(timezone.utc).isoformat(),
        "packet_failure_id": packet.get("failure_id"),
        "packet_schema": packet.get("schema"),
        "model": model,
        "request_id": response.get("id"),
        "usage": response.get("usage"),
        "answer": answer,
    }
    json_path = out_dir / f"root-cause-{stem}.json"
    json_path.write_text(json.dumps(record, indent=2) + "\n")

    md = [
        f"# Root cause candidate — {packet.get('failure_id')}",
        "",
        f"- invariant: `{packet.get('invariant') or packet.get('first_error') or 'unknown'}`",
        f"- model: `{record['model']}`",
        f"- stored: {record['stored_at']}",
        "",
        "## Ranked causes",
        "",
    ]
    for cause in answer.get("causes", []):
        md.append(
            "- `{symbol}` ({file}) — confidence {confidence}: {reasoning}".format(
                symbol=cause.get("symbol"),
                file=cause.get("file", "unknown"),
                confidence=cause.get("confidence"),
                reasoning=cause.get("reasoning", ""),
            )
        )
    for key, title in (
        ("first_experiment", "First experiment"),
        ("required_regression_test", "Required regression test"),
        ("notes", "Notes"),
    ):
        if answer.get(key):
            md += ["", f"## {title}", "", str(answer[key])]
    md += [
        "",
        "## Replay",
        "",
        "```bash",
        str(packet.get("replay_command", "unknown")),
        "```",
        "",
        "Candidates are ranked hypotheses from a model, not verified causes.",
        "Verify before changing code.",
    ]
    (out_dir / f"root-cause-{stem}.md").write_text("\n".join(md) + "\n")
    return json_path


def store_invalid(packet: dict, out_dir: Path, raw: str, reason: str) -> int:
    """Keep the raw answer and return the contract-failure exit code.

    Used for every failure after something answered: malformed JSON, a wrong
    envelope shape, an empty completion, or a non-conforming contract. The
    answer is preserved so a human can see exactly what the router said.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    raw_path = (
        out_dir
        / f"root-cause-{_safe_component(str(packet.get('failure_id', 'unknown')))}.invalid.txt"
    )
    raw_path.write_text(raw if isinstance(raw, str) else json.dumps(raw, indent=2, default=str))
    print(
        f"root_cause: {reason}; nothing was stored, raw answer kept at {raw_path}",
        file=sys.stderr,
    )
    return 3


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path)
    parser.add_argument("--router", default=os.environ.get("X3_ROUTER_URL", DEFAULT_ROUTER))
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--timeout", type=float, default=180.0)
    # Reasoning models bill their thinking against this budget and answer
    # nothing when it runs out, so the default leaves room for both.
    parser.add_argument("--max-tokens", type=int, default=4000)
    parser.add_argument("--out", type=Path, default=Path("reports/root-cause"))
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print the request payload and exit without contacting the router",
    )
    args = parser.parse_args(argv)

    try:
        packet = load_packet(args.packet)
    except SystemExit as exc:
        print(exc, file=sys.stderr)
        return 2

    payload = request_payload(packet, args.model, args.max_tokens)
    if args.dry_run:
        print(json.dumps(payload, indent=2))
        return 0

    request = urllib.request.Request(
        args.router.rstrip("/") + "/v1/chat/completions",
        json.dumps(payload).encode(),
        {"Content-Type": "application/json", "X-X3-Agent": "x3-root-cause"},
    )
    token = os.environ.get("X3_ROUTER_TOKEN")
    if token:
        request.add_header("Authorization", "Bearer " + token)
    try:
        with urllib.request.urlopen(request, timeout=args.timeout) as response:
            raw_body = response.read()
    except urllib.error.HTTPError as exc:
        # The router answered (with an error status), so this is a bad
        # answer, not an unreachable router: preserve it and exit 3.
        try:
            payload = exc.read().decode(errors="replace")
        except Exception:
            payload = f"HTTP {exc.code}"
        return store_invalid(packet, args.out, payload, f"router answered HTTP {exc.code}")
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        print(
            f"root_cause: router at {args.router} did not answer ({exc}); "
            "nothing was stored and no cause was invented",
            file=sys.stderr,
        )
        return 2

    text = raw_body.decode(errors="replace")
    try:
        body = json.loads(text)
    except ValueError as exc:
        return store_invalid(packet, args.out, text, f"answer is not JSON: {exc}")
    if not isinstance(body, dict):
        return store_invalid(packet, args.out, text, "answer is not a JSON object")
    choices = body.get("choices")
    if not isinstance(choices, list) or not choices:
        return store_invalid(packet, args.out, text, "answer has no 'choices' list")
    choice = choices[0]
    if not isinstance(choice, dict):
        return store_invalid(packet, args.out, text, "choice is not a JSON object")
    message = choice.get("message")
    content = message.get("content") if isinstance(message, dict) else None
    if not isinstance(content, str):
        content = ""
    if not content:
        finish_reason = choice.get("finish_reason")
        detail = "answer contained no content"
        if finish_reason == "length":
            detail += (
                " (finish_reason=length: the budget was spent before any content, "
                "often on reasoning tokens; raise --max-tokens)"
            )
        return store_invalid(
            packet,
            args.out,
            "empty content, finish_reason={}; full response:\n{}".format(
                finish_reason, json.dumps(body, indent=2)
            ),
            detail,
        )
    try:
        answer = extract_contract(content)
    except ValueError as exc:
        return store_invalid(packet, args.out, content, f"answer did not match the contract ({exc})")

    stored = store(packet, answer, body, body.get("model", args.model), args.out)
    print(f"root_cause: stored {stored}")
    for cause in answer.get("causes", []):
        print(
            "  {symbol} ({file}) confidence={confidence}".format(
                symbol=cause.get("symbol"),
                file=cause.get("file", "unknown"),
                confidence=cause.get("confidence"),
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
