#!/usr/bin/env python3
"""Small OpenAI-compatible, budgeted model router. Standard library only."""
import argparse
import base64
import datetime as dt
import html
import json
import os
import re
import sqlite3
import subprocess
import sys
import threading
import time
import uuid
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

CRITICAL = ("consensus", "finality", "settlement", "atomic", "cryptograph", "supply", "runtime upgrade", "slashing", "cross-vm")
MAX_BODY = 2_000_000
# A single completion may not ask for more output than this. The bound exists so
# the per-request reservation is an upper bound on what the provider can bill.
MAX_OUTPUT_TOKENS = 32_768
DEFAULT_OUTPUT_TOKENS = 4_096
# Endpoints OpenAI clients probe that this router deliberately does not
# implement. A 501 names the gap; a 404 reads as a wrong base URL.
UNSUPPORTED_PATHS = ("/v1/embeddings", "/v1/audio")
# Responses tool types with no Chat Completions equivalent. They are refused by
# name rather than dropped: a tool that silently disappears makes the model look
# like it chose not to call something it was never offered.
UNSUPPORTED_TOOL_TYPES = ("file_search", "image_generation", "computer_use",
                          "code_interpreter", "mcp", "local_shell", "shell")
# ── Provider protocol capability ─────────────────────────────────────────
# Each provider declares the upstream wire protocol it speaks. The router never
# infers this from the base URL or the model name: a guessed protocol turns a
# provider's own errors into router bugs, and the two protocols are not
# interchangeable (a Responses body sent to /chat/completions is a 400).
PROTOCOL_RESPONSES = "responses"
PROTOCOL_CHAT = "chat_completions"
PROTOCOLS = (PROTOCOL_RESPONSES, PROTOCOL_CHAT)
# Responses tool types the router understands structurally and keeps for any
# provider that accepts the protocol. Everything else is a hosted tool a
# provider only gets when it declares it in `hosted_tools`.
NATIVE_TOOL_TYPES = ("function", "custom")
HOSTED_TOOL_TYPES = ("web_search", "file_search", "computer_use", "code_interpreter",
                     "mcp", "image_generation", "local_shell", "shell")
# The single Chat Completions argument that carries a custom (freeform) tool's
# body. Chat Completions has no freeform tool type, so the text travels as one
# string argument and is lifted back out on the way to the client.
CUSTOM_TOOL_INPUT = "input"
TOOL_CAPABLE = "supports_tools"
# A probe that cannot be answered inside its token budget is not evidence of
# anything. Thinking models can spend an entire small budget before they emit
# the call, so providers may raise this with `tool_probe_max_tokens`.
PROBE_MAX_TOKENS = 64
# Credential shapes scrubbed out of any provider error before it reaches a log
# line, a client, or the provider-health table.
SECRET_PATTERNS = (
    re.compile(r"sk-[A-Za-z0-9_\-]{8,}"),
    re.compile(r"(?i)(bearer\s+)[A-Za-z0-9._\-]{8,}"),
    re.compile(r"(?i)((?:api[_-]?key|authorization|token)\s*[=:]\s*)\S+"),
)
# A model can advertise tool support and still answer with JSON in the message
# body. The only evidence that counts is a `tool_calls` array, so the probe asks
# for one and will not take prose for an answer.
PROBE_TOOL = "x3_capability_probe"
PROBE_ARGUMENT = "value"
# ── Task classification (§2) ─────────────────────────────────────────────
# A deterministic keyword classifier, not a model call. Routing decisions have
# to be reproducible and free: asking a model which model to use would add the
# cost, the latency and the nondeterminism this router exists to manage. The
# table is the spec's class list, and every class is reachable and tested.
#
# Terms are matched against the task text only — user, assistant and tool
# messages — never the system prompt. Codex sends ~17KB of instructions that
# mention security, performance and testing in the abstract, and scoring those
# would classify everything as everything.
TASK_CLASSES = (
    "REPOSITORY_SEARCH",
    "SIMPLE_EDIT",
    "BOILERPLATE",
    "DOCUMENTATION",
    "TEST_GENERATION",
    "RUST_IMPLEMENTATION",
    "COMPILER_WORK",
    "CONSENSUS",
    "CRYPTOGRAPHY",
    "SECURITY_ANALYSIS",
    "FUZZING",
    "DEBUGGING",
    "ARCHITECTURE",
    "PERFORMANCE",
    "DATABASE",
    "NETWORKING",
    "EVM",
    "SVM",
    "X3VM",
    "X3_LANG",
    "CROSS_CHAIN",
    "CODE_REVIEW",
    "FAILURE_ANALYSIS",
)

CLASS_TERMS = {
    "REPOSITORY_SEARCH": ("where is", "find the", "search for", "locate", "grep", "which file",
                          "list files", "rg "),
    "SIMPLE_EDIT": ("rename", "typo", "one-line", "one line", "bump the version", "add a comment"),
    "BOILERPLATE": ("scaffold", "boilerplate", "stub out", "generate the skeleton", "new crate"),
    "DOCUMENTATION": ("readme", "document", "docs", "docstring", "comment the", "changelog"),
    "TEST_GENERATION": ("add a test", "write tests", "unit test", "test coverage", "regression test",
                        "add tests"),
    "RUST_IMPLEMENTATION": ("rust", "cargo", "crate", "trait impl", "implement the", "borrow checker",
                            "fn ", "pub struct"),
    "COMPILER_WORK": ("compiler", "parser", "lexer", "type check", "hir", "mir", "codegen",
                      "bytecode", "lowering", "ast"),
    "CONSENSUS": ("consensus", "finality", "finalize", "quorum", "validator set", "fork choice",
                  "equivocat", "slashing", "babe", "grandpa"),
    "CRYPTOGRAPHY": ("cryptograph", "signature", "ed25519", "secp256k1", "hash lock", "merkle",
                     "preimage", "key derivation", "aead"),
    "SECURITY_ANALYSIS": ("security", "exploit", "attack", "vulnerab", "audit", "adversar",
                          "threat model", "reentrancy", "fail closed", "privilege"),
    "FUZZING": ("fuzz", "corpus", "coverage-guided", "cargo-fuzz", "afl", "honggfuzz"),
    # "failing"/"broken"/"not working" earn their place: a live request saying
    # "the coordinator refund is failing again after a claim" carries no other
    # debugging marker and classified as DOCUMENTATION without them.
    "DEBUGGING": ("debug", "why does", "reproduce", "root cause", "stack trace", "panic",
                  "failing test", "bisect", "failing", "is broken", "not working",
                  "fails on", "error when", "regression in"),
    "ARCHITECTURE": ("architecture", "design the", "trade-off", "tradeoff", "refactor the module",
                     "restructure", "plan the"),
    "PERFORMANCE": ("performance", "benchmark", "throughput", "latency", "tps", "profil",
                    "optimize", "bottleneck", "regression benchmark"),
    "DATABASE": ("database", "sql", "sqlite", "migration", "schema", "index the", "rocksdb"),
    "NETWORKING": ("network", "p2p", "gossip", "libp2p", "peer", "bandwidth", "packet loss",
                   "partition", "tcp", "socket"),
    "EVM": ("evm", "solidity", "foundry", "ethereum", "abi", "gas ", "smart contract"),
    "SVM": ("svm", "solana", "anchor", "pda", "invoke_signed", "bpf"),
    "X3VM": ("x3vm", "x3-vm", "x3 virtual machine"),
    "X3_LANG": (".x3", "x3lang", "x3-lang", "x3 language", "native x3 language"),
    "CROSS_CHAIN": ("cross-chain", "cross chain", "cross-vm", "cross vm", "bridge", "relayer",
                    "htlc", "atomic swap", "light client"),
    "CODE_REVIEW": ("review", "critique", "look over", "second opinion", "check my patch"),
    "FAILURE_ANALYSIS": ("post-mortem", "postmortem", "incident", "failure analysis", "retrospective",
                         "what went wrong"),
}

# Classes that make a request safety-critical regardless of the words used:
# they carry economic or consensus meaning, and a third party must not see them
# unless an operator has cleared it.
CRITICAL_CLASSES = ("CONSENSUS", "CRYPTOGRAPHY", "SECURITY_ANALYSIS", "CROSS_CHAIN",
                    "FAILURE_ANALYSIS")

VERIFICATION_BY_CLASS = {
    "DOCUMENTATION": (),
    "REPOSITORY_SEARCH": (),
    "SIMPLE_EDIT": ("unit",),
    "BOILERPLATE": ("unit",),
    "TEST_GENERATION": ("unit",),
    "RUST_IMPLEMENTATION": ("unit", "integration"),
    "COMPILER_WORK": ("unit", "integration"),
    "CONSENSUS": ("unit", "integration", "local-ci", "audit"),
    "CRYPTOGRAPHY": ("unit", "integration", "audit"),
    "SECURITY_ANALYSIS": ("unit", "integration", "audit"),
    "FUZZING": ("unit", "fuzz"),
    "DEBUGGING": ("unit", "reproduction"),
    "ARCHITECTURE": ("unit", "integration", "review"),
    "PERFORMANCE": ("benchmark", "regression"),
    "DATABASE": ("unit", "integration", "migration"),
    "NETWORKING": ("unit", "integration"),
    "EVM": ("unit", "integration"),
    "SVM": ("unit", "integration"),
    "X3VM": ("unit", "integration"),
    "X3_LANG": ("unit", "integration", "conformance"),
    "CROSS_CHAIN": ("unit", "integration", "local-ci", "audit"),
    "CODE_REVIEW": ("review",),
    "FAILURE_ANALYSIS": ("reproduction", "regression"),
}

PARALLEL_BY_CLASS = {
    "REPOSITORY_SEARCH": "high",
    "DOCUMENTATION": "high",
    "TEST_GENERATION": "high",
    "BOILERPLATE": "high",
    "SIMPLE_EDIT": "high",
    "FUZZING": "high",
    "RUST_IMPLEMENTATION": "medium",
    "COMPILER_WORK": "medium",
    "DEBUGGING": "medium",
    "PERFORMANCE": "medium",
    "DATABASE": "medium",
    "EVM": "medium",
    "SVM": "medium",
    "X3VM": "medium",
    "X3_LANG": "medium",
    "NETWORKING": "low",
    "CONSENSUS": "low",
    "CRYPTOGRAPHY": "low",
    "SECURITY_ANALYSIS": "low",
    "CROSS_CHAIN": "low",
    "ARCHITECTURE": "low",
    "CODE_REVIEW": "high",
    "FAILURE_ANALYSIS": "low",
}

# Which logical model a class routes to when the client asks for `x3-auto`.
DEFAULT_CLASS_ROUTES = {
    "SECURITY_ANALYSIS": "x3-security",
    "CRYPTOGRAPHY": "x3-security",
    "CONSENSUS": "x3-security",
    "CROSS_CHAIN": "x3-security",
    "FAILURE_ANALYSIS": "x3-deep",
    "DEBUGGING": "x3-deep",
    "ARCHITECTURE": "x3-deep",
    "PERFORMANCE": "x3-deep",
    "CODE_REVIEW": "x3-review",
    "DOCUMENTATION": "x3-fast",
    "REPOSITORY_SEARCH": "x3-fast",
    "SIMPLE_EDIT": "x3-fast",
    "BOILERPLATE": "x3-fast",
}
DEFAULT_LOGICAL_MODEL = "x3-code"

# Ordered provider preference per logical model. The first *usable* provider
# in the list wins; capability, health, budget and privacy all still apply.
DEFAULT_POLICIES = {
    "x3-auto": {"tier": "auto",
                "order": ["deepseek", "openrouter", "ollama", "nemotron_lightning_free", "direct"]},
    "x3-fast": {"tier": "routine", "order": ["ollama", "deepseek", "openrouter"]},
    "x3-code": {"tier": "routine", "order": ["deepseek", "openrouter", "ollama"]},
    "x3-deep": {"tier": "routine", "order": ["deepseek", "openrouter", "direct"]},
    "x3-security": {"tier": "critical", "order": ["deepseek", "direct"]},
    "x3-review": {"tier": "routine", "order": ["deepseek", "openrouter"]},
    "x3-local": {"tier": "routine", "order": ["ollama"]},
}

# Terms that make a request critical whatever else the classifier thinks.
CRITICAL_TERMS = CRITICAL

# Statuses that mean "try again", as opposed to "this request is wrong".
# Retrying a 400 or a 401 only spends money to get the same answer.
RETRYABLE_STATUS = frozenset({408, 409, 425, 429, 500, 502, 503, 504})


def input_item_text(item):
    """The text a single Responses `input` item carries, if any."""
    if not isinstance(item, dict):
        return []
    parts = []
    content = item.get("content")
    if isinstance(content, str):
        parts.append(content)
    elif isinstance(content, list):
        parts.extend(part.get("text", "") for part in content
                     if isinstance(part, dict) and isinstance(part.get("text"), str))
    if item.get("type") == "custom_tool_call" and isinstance(item.get("input"), str):
        parts.append(item["input"])
    if isinstance(item.get("output"), str):
        parts.append(item["output"])
    return parts


def responses_input_items(request):
    """The Responses `input` as a list of items.

    The Responses API accepts `input` as a plain string as well as a list of
    items; a string stands for one user message. Codex sends a list, but the
    string form is part of the protocol and a client that uses it must not be
    turned away with "Expected an input list".
    """
    value = request.get("input")
    if isinstance(value, str):
        return [{"type": "message", "role": "user",
                 "content": [{"type": "input_text", "text": value}]}]
    return value if isinstance(value, list) else []


def task_text(request):
    """The part of a request that describes the work, for either wire surface.

    A Chat Completions request carries the work in `messages`; system and
    developer messages are excluded there because they restate the agent's own
    instructions and would swamp the signal from the task.

    A Responses request carries the work in `input` and `instructions`. The
    `instructions` string is included on purpose: it is exactly where Codex
    puts the project brief, and a consensus or slashing phrase written there
    has to reach the classifier, or a critical request can be routed as
    routine. Only `messages` can be excluded, so the two rules do not conflict.
    """
    parts = []
    instructions = request.get("instructions")
    if isinstance(instructions, str):
        parts.append(instructions)
    for item in responses_input_items(request):
        parts.extend(input_item_text(item))
    for message in request.get("messages") or []:
        if not isinstance(message, dict):
            continue
        if message.get("role") in ("system", "developer"):
            continue
        content = message.get("content")
        if isinstance(content, str):
            parts.append(content)
        elif isinstance(content, list):
            parts.extend(part.get("text", "") for part in content if isinstance(part, dict))
    for tool in request.get("tools") or []:
        if isinstance(tool, dict):
            function = tool.get("function") or {}
            if function.get("name"):
                parts.append(function["name"])
            elif isinstance(tool.get("name"), str):
                parts.append(tool["name"])
    return " ".join(parts).lower()


def classify(request):
    """Classify a request and estimate what it will cost to do safely.

    Returns the class, the scores behind it, and the estimates §2 asks for:
    complexity, risk, blast radius, context requirement, verification
    requirement and parallelizability.
    """
    text = task_text(request)
    words = re.findall(r"[a-z0-9_]+", text)
    joined = " " + " ".join(words) + " "
    # The keyword table matches on word-split text, where "cross-vm" has become
    # "cross vm". Critical terms are matched against both forms so a hyphenated
    # spelling ("cross-vm") is not silently treated as routine.
    hyphenated = " " + " ".join(re.findall(r"[a-z0-9_\-]+", text)) + " "

    scores = {}
    for name in TASK_CLASSES:
        total = 0
        for term in CLASS_TERMS.get(name, ()):
            needle = term.strip()
            if not needle:
                continue
            if " " in needle:
                if needle in joined:
                    total += 2
            elif " " + needle in joined or needle.endswith(" ") and needle in joined:
                total += 2
            elif needle in joined:
                # Prefix match, so "cryptograph" covers cryptographic and
                # cryptography without listing both.
                total += 1
        if total:
            scores[name] = total

    # Ties break on the fixed class order, so the same request always classifies
    # the same way.
    task_class = None
    best = 0
    for name in TASK_CLASSES:
        if scores.get(name, 0) > best:
            best = scores[name]
            task_class = name

    critical_terms = [term for term in CRITICAL_TERMS if term in joined or term in hyphenated]
    blast_radius = sorted(set(critical_terms))
    risk = "critical" if (critical_terms or task_class in CRITICAL_CLASSES) else "low"
    if risk != "critical" and task_class in ("ARCHITECTURE", "DATABASE", "COMPILER_WORK",
                                             "RUST_IMPLEMENTATION", "PERFORMANCE"):
        risk = "medium"

    context_tokens = len(json.dumps(request, ensure_ascii=False).encode("utf-8")) // 4
    distinct = len(scores)
    complexity = "low"
    if context_tokens > 40_000 or distinct >= 3:
        complexity = "high"
    elif context_tokens > 8_000 or distinct >= 2 or task_class in (
            "ARCHITECTURE", "CONSENSUS", "CROSS_CHAIN", "COMPILER_WORK", "CRYPTOGRAPHY"):
        complexity = "medium"

    if task_class is None:
        # Nothing in the table matched. Fall back to the shape of the request
        # rather than inventing a class: an agent request with tools is a code
        # task, anything else is a chat turn.
        task_class = "RUST_IMPLEMENTATION" if request.get("tools") else "DOCUMENTATION"

    return {
        "task_class": task_class,
        "scores": dict(sorted(scores.items(), key=lambda item: (-item[1], item[0]))),
        "complexity": complexity,
        "risk": risk,
        "blast_radius": blast_radius,
        "context_tokens": context_tokens,
        "verification": list(VERIFICATION_BY_CLASS.get(task_class, ("unit",))),
        "parallelizable": PARALLEL_BY_CLASS.get(task_class, "medium"),
        "critical_terms": critical_terms,
    }


class UnsupportedFeature(Exception):
    """The client asked for something Chat Completions cannot carry."""


class ClientDisconnected(Exception):
    """The caller closed the socket mid-stream. Not a provider failure."""


def _flatten_tools(tools, custom, disabled):
    """Chat Completions tools for a Responses `tools` list.

    `custom` collects the names of freeform tools so the answer can be turned
    back into `custom_tool_call` items, and `disabled` collects tools the client
    itself switched off.
    """
    translated = []
    for tool in tools or []:
        if not isinstance(tool, dict):
            continue
        kind = tool.get("type")
        if kind == "namespace":
            translated.extend(_flatten_tools(tool.get("tools"), custom, disabled))
            continue
        if kind == "function":
            if not isinstance(tool.get("name"), str) or not tool["name"]:
                raise UnsupportedFeature("a function tool without a name")
            function = {"name": tool["name"],
                        "parameters": tool.get("parameters") or {"type": "object", "properties": {}}}
            if tool.get("description"):
                function["description"] = tool["description"]
            translated.append({"type": "function", "function": function})
            continue
        if kind == "custom":
            name = tool.get("name")
            if not isinstance(name, str) or not name:
                raise UnsupportedFeature("a custom tool without a name")
            # The freeform body is carried verbatim in one string argument and
            # rebuilt as a `custom_tool_call` on the way back, so the client
            # never sees the difference. What is lost is the grammar, which
            # Chat Completions cannot express; the client still validates what
            # it receives against its own schema.
            custom.add(name)
            description = tool.get("description") or ""
            translated.append({"type": "function", "function": {
                "name": name,
                "description": (description + " " if description else "") +
                               "Pass the complete tool input verbatim as the `" + CUSTOM_TOOL_INPUT + "` string.",
                "parameters": {"type": "object", "required": [CUSTOM_TOOL_INPUT],
                               "properties": {CUSTOM_TOOL_INPUT: {
                                   "type": "string",
                                   "description": "The complete input for this tool, verbatim."}}},
            }})
            continue
        if kind == "web_search":
            if tool.get("external_web_access"):
                raise UnsupportedFeature("web_search with external_web_access enabled")
            # The client disabled it, so there is nothing to run and nothing to
            # misreport. A search that would actually leave the machine is
            # refused above rather than silently ignored.
            disabled.append("web_search")
            continue
        if kind in UNSUPPORTED_TOOL_TYPES:
            raise UnsupportedFeature("tool type " + str(kind))
    return translated


def flattened_tools(request):
    """(chat tools, freeform tool names, client-disabled tool names)."""
    custom, disabled = set(), []
    return _flatten_tools(request.get("tools"), custom, disabled), custom, disabled


def tool_choice_forces_a_tool(choice):
    """True when the caller named a tool or demanded a call."""
    return choice == "required" or (isinstance(choice, dict) and choice.get("type") in ("function", "custom"))


def chat_tool_choice(choice, custom):
    """Responses `tool_choice` -> Chat Completions `tool_choice`.

    A string passes through. The named-function object form becomes the Chat
    Completions object; dropping it made the router answer with whatever tool
    the model preferred while the client believed it had named one.
    """
    if choice is None:
        return None
    if isinstance(choice, str):
        if choice in ("auto", "none", "required"):
            return choice
        raise UnsupportedFeature("tool_choice " + choice)
    if isinstance(choice, dict):
        kind, name = choice.get("type"), choice.get("name")
        if kind in ("function", "custom") and isinstance(name, str) and name:
            return {"type": "function", "function": {"name": name}}
        if kind == "allowed_tools":
            mode = choice.get("mode")
            return mode if mode in ("auto", "required") else "auto"
    raise UnsupportedFeature("tool_choice " + json.dumps(choice)[:80])


def custom_tool_input(arguments):
    """The freeform body back out of the single-argument Chat encoding."""
    try:
        parsed = json.loads(arguments)
    except (TypeError, ValueError):
        return arguments or ""
    if isinstance(parsed, dict) and isinstance(parsed.get(CUSTOM_TOOL_INPUT), str):
        return parsed[CUSTOM_TOOL_INPUT]
    return arguments or ""


def apply_provider_reasoning(payload, provider):
    """Provider-specific reasoning rules for a tool turn.

    DeepSeek runs thinking mode by default and rejects a named or required
    `tool_choice` there with HTTP 400 ("Thinking mode does not support this
    tool_choice"). A provider that declares the conflict has thinking switched
    off for exactly those turns; every other turn keeps the provider default.

    The client's reasoning effort rides along only to a provider that has
    declared it accepts the parameter, so an unknown field is not forwarded to
    every other provider in the chain.
    """
    thinking = provider.get("thinking")
    if thinking and thinking.get("disable_when_tool_choice_forced") \
            and tool_choice_forces_a_tool(payload.get("tool_choice")):
        payload[thinking.get("parameter", "thinking")] = thinking.get("disabled_value", {"type": "disabled"})
    if not provider.get("reasoning_effort"):
        payload.pop("reasoning_effort", None)


def tool_probe_request(model, max_tokens=PROBE_MAX_TOKENS):
    """A request whose only purpose is to be answered with a tool call."""
    return {
        "model": model, "stream": False, "max_tokens": max_tokens, "tool_choice": "required",
        "tools": [{"type": "function", "function": {
            "name": PROBE_TOOL, "description": "Report a capability value.",
            "parameters": {"type": "object", "required": [PROBE_ARGUMENT],
                           "properties": {PROBE_ARGUMENT: {"type": "string"}}}}}],
        "messages": [{"role": "user",
                      "content": "Call the " + PROBE_TOOL + " tool with value set to \"ok\". "
                                 "Do not answer in text."}],
    }


def responses_messages(request):
    """Translate a Responses API request into Chat Completions messages.

    Codex sends the Responses shape: a top-level `instructions` string plus an
    `input` list whose items are messages, function calls, custom tool calls and
    their outputs. Each has a direct Chat Completions equivalent, so a tool
    round trip keeps the same call IDs and names on both sides.
    """
    messages = []
    instructions = request.get("instructions")
    if isinstance(instructions, str) and instructions:
        messages.append({"role": "system", "content": instructions})
    for item in responses_input_items(request):
        if not isinstance(item, dict):
            continue
        kind = item.get("type", "message")
        if kind == "message":
            role = item.get("role", "user")
            if role in ("developer",):
                role = "system"
            if role not in ("system", "user", "assistant"):
                role = "user"
            text = "".join(part.get("text", "") for part in item.get("content") or [] if isinstance(part, dict))
            messages.append({"role": role, "content": text})
        elif kind == "function_call":
            messages.append({
                "role": "assistant",
                "content": None,
                "tool_calls": [{
                    "id": item.get("call_id", ""),
                    "type": "function",
                    "function": {"name": item.get("name", ""), "arguments": item.get("arguments") or "{}"},
                }],
            })
        elif kind == "custom_tool_call":
            messages.append({
                "role": "assistant",
                "content": None,
                "tool_calls": [{
                    "id": item.get("call_id", ""),
                    "type": "function",
                    "function": {"name": item.get("name", ""),
                                 "arguments": json.dumps({CUSTOM_TOOL_INPUT: item.get("input") or ""})},
                }],
            })
        elif kind == "function_call_output":
            messages.append({
                "role": "tool",
                "tool_call_id": item.get("call_id", ""),
                "content": str(item.get("output", "")),
            })
        elif kind == "custom_tool_call_output":
            messages.append({
                "role": "tool",
                "tool_call_id": item.get("call_id", ""),
                "content": str(item.get("output", "")),
            })
    return messages


def responses_request(request):
    """(Chat Completions body, freeform tool names, client-disabled tool names)."""
    chat = {"messages": responses_messages(request)}
    tools, custom, disabled = flattened_tools(request)
    if tools:
        chat["tools"] = tools
        choice = chat_tool_choice(request.get("tool_choice"), custom)
        if choice is not None:
            chat["tool_choice"] = choice
        if isinstance(request.get("parallel_tool_calls"), bool):
            chat["parallel_tool_calls"] = request["parallel_tool_calls"]
    limit = request.get("max_output_tokens")
    if isinstance(limit, int) and limit > 0:
        chat["max_tokens"] = limit
    # Codex sends the effort it was asked for. It is carried here and dropped
    # again in `provider_payload` unless the provider declares it accepts it.
    reasoning = request.get("reasoning")
    if isinstance(reasoning, dict) and isinstance(reasoning.get("effort"), str):
        chat["reasoning_effort"] = reasoning["effort"]
    return chat, custom, disabled


def responses_request_to_chat(request):
    """A complete Chat Completions request for the provider loop to execute."""
    return responses_request(request)[0]


def valid_completion_message(message):
    """Validate the assistant payload before accepting upstream success."""
    if not isinstance(message, dict) or message.get("role") != "assistant":
        return False
    content, refusal, calls = message.get("content"), message.get("refusal"), message.get("tool_calls")
    if content is not None and not isinstance(content, str):
        return False
    if refusal is not None and not isinstance(refusal, str):
        return False
    if calls is not None:
        if not isinstance(calls, list):
            return False
        ids = set()
        for call in calls:
            if not isinstance(call, dict) or call.get("type") != "function":
                return False
            identifier, function = call.get("id"), call.get("function")
            if (not isinstance(identifier, str) or not identifier or identifier in ids
                    or not isinstance(function, dict)
                    or not isinstance(function.get("name"), str) or not function["name"]
                    or not isinstance(function.get("arguments"), str)):
                return False
            ids.add(identifier)
    return bool(content or refusal or calls)


def chat_message_to_response_output(message, prefix, custom=frozenset()):
    """Chat Completions message -> the Responses `output` list."""
    output = []
    content = message.get("content")
    parts = []
    if content:
        parts.append({"type": "output_text", "text": content, "annotations": []})
    if message.get("refusal"):
        parts.append({"type": "refusal", "refusal": message["refusal"]})
    if parts:
        output.append({"id": prefix + "msg", "type": "message", "role": "assistant", "status": "completed",
                       "content": parts})
    for index, call in enumerate(message.get("tool_calls") or []):
        function = call.get("function") or {}
        name = function.get("name", "")
        arguments = function.get("arguments") or ""
        if name in custom:
            output.append({"id": f"{prefix}fc{index}", "type": "custom_tool_call", "status": "completed",
                           "call_id": call.get("id", ""), "name": name,
                           "input": custom_tool_input(arguments)})
        else:
            output.append({"id": f"{prefix}fc{index}", "type": "function_call", "status": "completed",
                           "call_id": call.get("id", ""), "name": name,
                           "arguments": arguments or "{}"})
    return output


def responses_envelope(response_id, model, output, usage=None, status="completed"):
    envelope = {"id": response_id, "object": "response", "created_at": int(time.time()),
                "status": status, "model": model, "output": output,
                "parallel_tool_calls": False, "tool_choice": "auto", "tools": []}
    if usage is not None:
        normalized = normalize_usage(usage)
        envelope["usage"] = {"input_tokens": normalized["prompt_tokens"],
                             "output_tokens": normalized["completion_tokens"],
                             "total_tokens": normalized["total_tokens"]}
    return envelope


def responses_failure_event(response_id, model, message, status=None):
    """A terminal `response.failed` event for a stream the provider did not finish."""
    envelope = responses_envelope(response_id, model, [], None, status="failed")
    envelope["error"] = {"code": "upstream_error", "message": sanitize_secret(message)}
    if status is not None:
        envelope["error"]["status"] = status
    return {"type": "response.failed", "response": envelope}


class ResponsesEventReader:
    """Terminal state of a native Responses SSE stream.

    The Responses protocol terminates with a semantic event
    (`response.completed`, `response.incomplete`, `response.failed`), not
    `data: [DONE]`. Watching for `[DONE]` on this stream would read to the
    socket close and report a finished answer as unfinished.
    """

    TERMINAL_TYPES = ("response.completed", "response.incomplete", "response.failed")

    def __init__(self):
        self.usage = None
        self.model = None
        self.status = None
        self.failed = False
        self.terminal = False

    def begins(self, line):
        return line.startswith(b"event: ") or line.startswith(b"data: ")

    def stops(self, line):
        return self.terminal

    def first_event_type(self, line):
        """The type of the first event on a stream, or None if it is not one."""
        data = line[6:].strip() if line.startswith(b"data: ") else b""
        if not data or data == b"[DONE]":
            return None
        try:
            event = json.loads(data)
        except ValueError:
            return None
        return event.get("type") if isinstance(event, dict) else None

    def read(self, line):
        if not line.startswith(b"data: "):
            return
        data = line[6:].strip()
        if not data:
            return
        if data == b"[DONE]":
            self.terminal = True
            return
        try:
            event = json.loads(data)
        except ValueError:
            return
        if not isinstance(event, dict):
            return
        response = event.get("response")
        if isinstance(response, dict):
            if response.get("usage"):
                self.usage = response["usage"]
            if isinstance(response.get("model"), str):
                self.model = response["model"]
        if event.get("type") in self.TERMINAL_TYPES:
            self.terminal = True
            self.status = event["type"]
            self.failed = event["type"] == "response.failed"

    def terminal_reason(self):
        """None when the stream ended on a terminal event, else why not."""
        if self.terminal:
            return None
        return "stream_ended_without_a_terminal_event"


class ResponsesStream:
    """Re-emit the Chat Completions SSE that `Router.stream` forwards as Responses events.

    `Router.stream` already owns provider selection, budgets, cooldowns and
    fallback, and it hands every upstream line to the `send` callback. This
    adapter is that callback: it parses chat chunks and writes the Responses
    event sequence Codex expects, so the budget logic is not duplicated.
    """

    def __init__(self, send, response_id, model, custom=frozenset()):
        self.send = send
        self.response_id = response_id
        self.model = model
        self.custom = set(custom)
        self.started = False
        self.message_started = False
        self.text = []
        self.calls = {}          # tool index -> {"id","name","arguments"}
        self.call_order = []
        self.usage = None
        self.finish_reason = None
        self.saw_done = False
        self.finished = False

    def _emit(self, event):
        self.send(("data: " + json.dumps(event) + "\n\n").encode())

    def start(self):
        self.started = True
        self._emit({"type": "response.created",
                    "response": responses_envelope(self.response_id, self.model, [], status="in_progress")})

    def _open_message(self):
        if not self.message_started:
            self.message_started = True
            self._emit({"type": "response.output_item.added", "output_index": 0,
                        "item": {"id": self.response_id + "msg", "type": "message", "role": "assistant",
                                 "status": "in_progress", "content": []}})

    def _open_call(self, index, call_id, name):
        self.message_started = True
        # A freeform tool has no chat equivalent, so its body arrives as one
        # string argument. The item is opened with an empty `input` and closed
        # with the whole body rather than as fabricated argument fragments.
        freeform = name in self.custom
        if freeform:
            item = {"id": f"{self.response_id}fc{index}", "type": "custom_tool_call", "status": "in_progress",
                    "call_id": call_id, "name": name, "input": ""}
        else:
            item = {"id": f"{self.response_id}fc{index}", "type": "function_call", "status": "in_progress",
                    "call_id": call_id, "name": name, "arguments": ""}
        self._emit({"type": "response.output_item.added", "output_index": index + 1,
                    "item": item})

    def on_line(self, chunk):
        line = chunk.decode("utf-8", "replace").strip()
        if not line.startswith("data: "):
            return
        data = line[6:].strip()
        if data == "[DONE]":
            self.saw_done = True
            return
        if not data:
            return
        try:
            event = json.loads(data)
        except ValueError:
            return
        if isinstance(event.get("model"), str) and event["model"]:
            # Report the model that answered, not the alias the client sent.
            self.model = event["model"]
        if event.get("usage"):
            self.usage = event["usage"]
        choices = event.get("choices") or []
        if not choices:
            return
        if choices[0].get("finish_reason"):
            self.finish_reason = choices[0]["finish_reason"]
        delta = choices[0].get("delta") or {}
        text = delta.get("content")
        if text:
            self._open_message()
            self.text.append(text)
            self._emit({"type": "response.output_text.delta", "item_id": self.response_id + "msg",
                        "output_index": 0, "content_index": 0, "delta": text})
        for call in delta.get("tool_calls") or []:
            index = call.get("index", len(self.call_order))
            if index not in self.calls:
                self.calls[index] = {"id": call.get("id") or f"call_{index}", "name": "", "arguments": ""}
                self.call_order.append(index)
                function = call.get("function") or {}
                self.calls[index]["name"] = function.get("name", "")
                self._open_call(index, self.calls[index]["id"], self.calls[index]["name"])
            function = call.get("function") or {}
            if function.get("name"):
                self.calls[index]["name"] = function["name"]
            arguments = function.get("arguments")
            if arguments:
                self.calls[index]["arguments"] += arguments
                if self.calls[index]["name"] not in self.custom:
                    self._emit({"type": "response.function_call_arguments.delta",
                                "item_id": f"{self.response_id}fc{index}", "output_index": index + 1,
                                "delta": arguments})

    def finish(self):
        """Close the stream as completed or incomplete, never as a success lie."""
        if self.finished:
            return
        self.finished = True
        output = []
        if self.text:
            text = "".join(self.text)
            self._emit({"type": "response.output_text.done", "item_id": self.response_id + "msg",
                        "output_index": 0, "content_index": 0, "text": text})
            item = {"id": self.response_id + "msg", "type": "message", "role": "assistant", "status": "completed",
                    "content": [{"type": "output_text", "text": text, "annotations": []}]}
            self._emit({"type": "response.output_item.done", "output_index": 0, "item": item})
            output.append(item)
        for index in self.call_order:
            call = self.calls[index]
            freeform = call["name"] in self.custom
            if freeform:
                item = {"id": f"{self.response_id}fc{index}", "type": "custom_tool_call", "status": "completed",
                        "call_id": call["id"], "name": call["name"],
                        "input": custom_tool_input(call["arguments"])}
            else:
                item = {"id": f"{self.response_id}fc{index}", "type": "function_call", "status": "completed",
                        "call_id": call["id"], "name": call["name"], "arguments": call["arguments"] or "{}"}
                self._emit({"type": "response.function_call_arguments.done", "item_id": item["id"],
                            "output_index": index + 1, "arguments": item["arguments"]})
            self._emit({"type": "response.output_item.done", "output_index": index + 1, "item": item})
            output.append(item)
        if not output:
            self._open_message()
            output = chat_message_to_response_output({"content": ""}, self.response_id, self.custom)
        status, details = "completed", None
        if self.finish_reason == "length":
            status, details = "incomplete", {"reason": "max_output_tokens"}
        elif not self.saw_done and self.finish_reason is None:
            # No `[DONE]` and no finish reason means the upstream stopped
            # mid-answer. Reporting that as completed is the failure this
            # router shipped once with a text-only stream.
            status, details = "incomplete", {"reason": "stream_ended_without_a_terminal_event"}
        envelope = responses_envelope(self.response_id, self.model, output, self.usage, status=status)
        if details:
            envelope["incomplete_details"] = details
        self._emit({"type": "response.incomplete" if details else "response.completed", "response": envelope})

    def fail(self, message, status=None):
        """Terminal event for a stream the provider did not finish."""
        if self.finished:
            return
        self.finished = True
        envelope = responses_envelope(self.response_id, self.model, [], self.usage, status="failed")
        envelope["error"] = {"code": "upstream_error", "message": message}
        if status is not None:
            envelope["error"]["status"] = status
        self._emit({"type": "response.failed", "response": envelope})


def output_bound(request, config):
    """The largest completion this request can be billed for.

    The reservation is an upper bound, so it has to bound whichever output
    parameter the client actually set. A Responses request names it
    `max_output_tokens`, a Chat Completions request `max_tokens` (or
    `max_completion_tokens` for the models that moved to it). Reading only one
    of them reserved the 4096 default while the provider billed for whatever
    the request actually asked for.
    """
    for key in ("max_output_tokens", "max_completion_tokens", "max_tokens"):
        value = request.get(key)
        if type(value) is int:
            return value
    return config.get("default_max_output_tokens", DEFAULT_OUTPUT_TOKENS)


def request_error(request, config):
    """Reject a request whose cost the reservation would not bound.

    Two shapes defeat a per-request reservation: an output parameter the
    estimate does not read, and `n`/`best_of`, which multiply the completions
    the provider bills for while the estimate assumes exactly one.
    """
    for key in ("max_tokens", "max_completion_tokens", "max_output_tokens"):
        value = request.get(key)
        if value is not None and (type(value) is not int or not 1 <= value <= config.get("max_output_tokens", MAX_OUTPUT_TOKENS)):
            return "Invalid " + key
    for key in ("n", "best_of"):
        value = request.get(key)
        if value is not None and value != 1:
            return key + " must be 1 when set: one reservation covers one completion"
    if request.get("model") is not None and not isinstance(request.get("model"), str):
        return "Invalid model"
    return None


def pricing_error(provider):
    """Fail closed for paid providers with missing or old price assumptions."""
    if not provider.get("api_key_env"):
        return None
    if provider.get("free_model"):
        if not provider.get("model", "").endswith(":free") or provider.get("input_usd_per_million", 0) != 0 or provider.get("output_usd_per_million", 0) != 0:
            return "free model must use a :free ID and zero prices"
    elif provider.get("input_usd_per_million", 0) <= 0 or provider.get("output_usd_per_million", 0) <= 0:
        return "configure positive token prices"
    try:
        checked = dt.date.fromisoformat(provider["pricing_checked_on"])
        age = (dt.datetime.now(dt.timezone.utc).date() - checked).days
        if age < 0 or age > provider.get("pricing_max_age_days", 30):
            return "refresh provider pricing"
    except (KeyError, TypeError, ValueError):
        return "configure pricing_checked_on (YYYY-MM-DD)"
    return None


def may_serve_critical(provider):
    """Whether a provider may be handed a critical request.

    Critical requests carry consensus and settlement code. A provider that
    authenticates against a third party (`api_key_env` set) may only see one
    when it is explicitly cleared with `critical_allowed`; being named in
    `budget_fallback` is not clearance, because the default fallback includes
    free cloud models whose operator logs prompts. A provider with no
    credentials is talking to a model on this machine, so it cannot leak the
    request to anyone and stays eligible — that is how a critical request keeps
    working when a paid provider is unavailable or over budget.
    """
    return bool(provider.get("critical_allowed", False)) or not provider.get("api_key_env")


def provider_protocols(provider):
    """The upstream protocols a provider declares, never inferred.

    `protocols` lists everything the provider speaks. `protocol` names the one
    it speaks when it only speaks one. A provider that declares neither keeps
    the Chat Completions default that predates the field, so an older config
    still routes; every provider this router ships is explicit.
    """
    declared = provider.get("protocols")
    if isinstance(declared, list):
        protocols = tuple(name for name in declared if name in PROTOCOLS)
        if protocols:
            return protocols
    protocol = provider.get("protocol")
    if protocol in PROTOCOLS:
        return (protocol,)
    return (PROTOCOL_CHAT,)


def sanitize_secret(text):
    """Strip credential-shaped substrings from text headed for a log or client."""
    cleaned = str(text)
    for pattern in SECRET_PATTERNS:
        cleaned = pattern.sub(lambda match: (match.group(1) if match.groups() else "") + "[redacted]",
                              cleaned)
    return cleaned


def http_error_detail(exc, limit=200):
    """The provider's own error message, sanitized.

    "HTTP 400" says a request was rejected but not what to change. The body
    names the field; the body can also echo a credential if a client leaked one
    into a header, so everything read here is scrubbed before it can reach a
    log line, a failure response, or provider health.
    """
    try:
        raw = exc.read()
    except Exception:
        return ""
    text = raw.decode("utf-8", "replace").strip() if isinstance(raw, bytes) else str(raw)
    if not text:
        return ""
    message = None
    try:
        parsed = json.loads(text)
    except ValueError:
        parsed = None
    if isinstance(parsed, dict):
        error = parsed.get("error")
        if isinstance(error, dict):
            message = error.get("message")
        elif isinstance(error, str):
            message = error
        if not isinstance(message, str):
            message = parsed.get("message") if isinstance(parsed.get("message"), str) else None
    if not isinstance(message, str):
        message = text
    return sanitize_secret(message)[:limit]


def normalize_usage(usage):
    """Either wire shape -> the router's internal token names.

    Chat Completions reports `prompt_tokens`/`completion_tokens`; the Responses
    API reports `input_tokens`/`output_tokens`. Reading only the chat names
    silently booked zero for every token a Responses provider billed.
    """
    if not isinstance(usage, dict):
        return {}
    prompt = usage.get("prompt_tokens")
    if not isinstance(prompt, int):
        prompt = usage.get("input_tokens")
    completion = usage.get("completion_tokens")
    if not isinstance(completion, int):
        completion = usage.get("output_tokens")
    prompt = prompt if isinstance(prompt, int) else 0
    completion = completion if isinstance(completion, int) else 0
    normalized = dict(usage)
    normalized["prompt_tokens"] = prompt
    normalized["completion_tokens"] = completion
    if not isinstance(normalized.get("total_tokens"), int):
        normalized["total_tokens"] = prompt + completion
    return normalized


def is_responses_object(result):
    """Whether an upstream body is a Responses object rather than a chat one."""
    return (isinstance(result, dict) and result.get("object") == "response"
            and isinstance(result.get("output"), list))


def normalize_responses_tools(payload, provider):
    """Keep the tools a Responses provider really supports; drop the rest.

    Codex sends `function` tools, a freeform `custom` apply_patch, a
    `namespace` of collaboration tools, and hosted tools it may or may not have
    enabled. One hosted tool the provider does not implement must not sink the
    whole agent request, and it must not be forwarded as though it worked:
    namespaced tools are flattened into their members and unsupported hosted
    tools are dropped and reported by name.
    """
    tools = payload.get("tools")
    if not isinstance(tools, list):
        return []
    declared = set(provider.get("hosted_tools") or ())
    kept, dropped = [], []

    def walk(entries):
        for tool in entries or []:
            if not isinstance(tool, dict):
                continue
            kind = tool.get("type")
            if kind == "namespace":
                walk(tool.get("tools"))
            elif kind in NATIVE_TOOL_TYPES or kind in declared:
                kept.append(tool)
            else:
                dropped.append(str(kind))

    walk(tools)
    if not dropped:
        return []
    if kept:
        payload["tools"] = kept
    else:
        payload.pop("tools", None)
        # A forced choice with nothing left to call is a guaranteed 400, so it
        # is relaxed rather than sent.
        if payload.get("tool_choice") not in (None, "auto", "none"):
            payload["tool_choice"] = "auto"
    return dropped


class UpstreamRequest:
    """One inbound request, expressed in every protocol a provider may need.

    A Chat Completions request only has the chat shape. A Responses request
    keeps its native body so a provider that speaks the Responses protocol is
    handed it un-translated, and carries the Chat Completions translation
    alongside for the providers that only speak Chat Completions.
    """

    def __init__(self, surface, body, chat=None, responses=None,
                 custom=frozenset(), disabled=(), chat_error=None):
        self.surface = surface
        self.body = body
        self.chat = chat
        self.responses = responses
        self.custom = set(custom)
        self.disabled = list(disabled)
        self.chat_error = chat_error

    def protocol_for(self, provider):
        """The upstream protocol to use, or None when the provider cannot serve.

        The surface's own protocol always wins when the provider speaks it: a
        Responses request reaches a Responses provider natively. A Responses
        request falls back to the Chat Completions translation only for a
        provider that does not speak Responses. A Chat request is never handed
        to a Responses-only provider — that translation does not exist, and a
        clean refusal beats a malformed upstream request.
        """
        protocols = provider_protocols(provider)
        if self.surface in protocols:
            return self.surface
        if self.surface == PROTOCOL_RESPONSES and PROTOCOL_CHAT in protocols and self.chat is not None:
            return PROTOCOL_CHAT
        return None

    def supports(self, provider):
        return self.protocol_for(provider) is not None

    def refusal(self, provider):
        """Why `protocol_for` returned None, named for the attempt list."""
        protocols = provider_protocols(provider)
        if self.surface == PROTOCOL_RESPONSES and PROTOCOL_CHAT in protocols and self.chat is None:
            return "request has no Chat Completions form: " + str(self.chat_error)
        return "provider does not declare the " + self.surface + " protocol"

    def shape(self, protocol):
        return self.responses if protocol == PROTOCOL_RESPONSES else self.chat

    def control(self):
        """The dict used for classification, budget and capability checks."""
        return self.body

    def model(self, fallback="x3-auto"):
        value = self.body.get("model")
        return value if isinstance(value, str) else fallback


def coerce_plan(candidate):
    """Accept a bare request body as well as a prepared `UpstreamRequest`.

    The HTTP handler builds the full plan, because only it knows that a
    Responses request also has a Chat Completions translation. Direct callers
    hand in a body, and it is read as the shape it is written in:
    `input`/`instructions` is a Responses request, `messages` a Chat
    Completions one.
    """
    if isinstance(candidate, UpstreamRequest):
        return candidate
    if not isinstance(candidate, dict):
        raise TypeError("Expected a request mapping")
    if "input" in candidate and "messages" not in candidate:
        try:
            chat, custom, disabled = responses_request(candidate)
        except UnsupportedFeature as exc:
            return UpstreamRequest(PROTOCOL_RESPONSES, candidate, responses=candidate,
                                   chat_error=str(exc))
        return UpstreamRequest(PROTOCOL_RESPONSES, candidate, chat=chat, responses=candidate,
                               custom=custom, disabled=disabled)
    return UpstreamRequest(PROTOCOL_CHAT, candidate, chat=candidate)


class Router:
    def __init__(self, config, db_path):
        self.config = config
        self.lock = threading.Lock()
        # Requests currently running against each provider. In memory only:
        # it describes this process's load, which a restart resets anyway.
        self.in_flight = {}
        self.in_flight_lock = threading.Lock()
        self.context = threading.local()
        self.db = None
        self.reconciled_orphans = 0
        self.capabilities = {}
        # SQLite creates the file, but cannot create its parent directories.
        # Preserve special in-memory databases used by embedded callers/tests.
        db_path = os.fsdecode(db_path)
        try:
            if db_path and db_path != ":memory:":
                parent = os.path.dirname(os.path.abspath(db_path))
                os.makedirs(parent, exist_ok=True)
            self.db = sqlite3.connect(db_path, check_same_thread=False, timeout=30)
            # WAL lets the dashboard and a second router process read while a
            # request thread writes; the busy timeout absorbs short lock waits
            # instead of failing the request with "database is locked".
            self.db.execute("PRAGMA busy_timeout=30000")
            if db_path != ":memory:":
                self.db.execute("PRAGMA journal_mode=WAL")
            self.initialize_database()
            self.reconcile_reservations()
        except Exception as exc:
            if self.db is not None:
                self.db.close()
            raise RuntimeError(f"Could not initialize router database {db_path!r}: {exc}") from exc

    def initialize_database(self):
        """Create tables and migrate older router databases on startup."""
        self.db.execute("CREATE TABLE IF NOT EXISTS usage (day TEXT, agent TEXT, provider TEXT, model TEXT, input_tokens INTEGER, output_tokens INTEGER, cost_usd REAL)")
        if "task_id" not in {row[1] for row in self.db.execute("PRAGMA table_info(usage)")}:
            self.db.execute("ALTER TABLE usage ADD COLUMN task_id TEXT")
        self.db.execute("CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, agent TEXT, revision TEXT, scope TEXT, requests INTEGER DEFAULT 0, elapsed_ms REAL DEFAULT 0, outcome TEXT DEFAULT 'pending', evidence TEXT, in_flight INTEGER DEFAULT 0)")
        self.db.execute("CREATE TABLE IF NOT EXISTS reservations (id TEXT PRIMARY KEY, day TEXT, agent TEXT, cost_usd REAL)")
        # A reservation is only deleted by `finish`, which runs in the request
        # thread. Without a timestamp there is no way to tell one that is still
        # in flight from one whose process died, so the day's `reserved_usd`
        # could only ever grow.
        if "created_at" not in {row[1] for row in self.db.execute("PRAGMA table_info(reservations)")}:
            self.db.execute("ALTER TABLE reservations ADD COLUMN created_at REAL")
        self.db.execute("CREATE TABLE IF NOT EXISTS provider_health (provider TEXT PRIMARY KEY, failures INTEGER DEFAULT 0, cooldown_until REAL DEFAULT 0, last_error TEXT, last_failure_at REAL)")
        # Measured capability (§3). `provider_health` answers "is it working
        # right now"; this answers "how well has it worked", which is what a
        # routing decision needs and what a static config cannot know.
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS provider_stats ("
            "provider TEXT, model TEXT, attempts INTEGER DEFAULT 0, successes INTEGER DEFAULT 0, "
            "failures INTEGER DEFAULT 0, retries INTEGER DEFAULT 0, "
            "latency_ms_total REAL DEFAULT 0, latency_samples INTEGER DEFAULT 0, "
            "input_tokens INTEGER DEFAULT 0, output_tokens INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0, "
            "PRIMARY KEY (provider, model))")
        self.db.commit()

    def choose(self, request):
        """Pick the tier, the provider order, and the reasoning behind them.

        The client's `model` is normally an alias Codex sends and the router
        ignores, but a real logical model name (`x3-security`, `x3-local`, ...)
        is a routing instruction and is honoured.

        A critical classification is a *floor*, never a ceiling: a policy
        cannot downgrade it. A request containing consensus or settlement terms
        stays critical even when its task class would otherwise route to the
        cheap chain, because that check is what keeps such code off third-party
        providers.
        """
        classification = classify(request)
        policies = self.config.get("policies") or DEFAULT_POLICIES
        class_routes = self.config.get("class_routes") or DEFAULT_CLASS_ROUTES

        requested = request.get("model")
        logical = requested if isinstance(requested, str) and requested in policies else None
        if logical is None:
            target = class_routes.get(classification["task_class"], DEFAULT_LOGICAL_MODEL)
            logical = target if target in policies else "x3-auto"

        policy = policies.get(logical) or policies.get("x3-auto") or {}
        tier = "critical" if (
            policy.get("tier") == "critical" or classification["risk"] == "critical"
        ) else "routine"

        order = [name for name in policy.get("order", []) if name in self.config["providers"]]
        if not order:
            # No usable policy — an older config, or one whose providers were
            # renamed. Fall back to the fixed route for the tier rather than
            # failing, so an existing deployment keeps working.
            routes = self.config.get("routes") or {}
            order = list(routes.get(tier, []))
        return tier, order, classification, logical

    def attempt_order(self, chain):
        """The providers to try, in order, for one request.

        `budget_fallback` is appended after the configured chain, so an
        exhausted budget or an unusable paid provider degrades the model
        instead of failing the request. Because it sits last, it is only
        reached when nothing better answered, and it can keep a critical
        request on this machine (a provider with no credentials cannot leak
        it) — but it never clears a third-party provider for critical work.
        See `may_serve_critical`.
        """
        order = list(chain)
        for name in self.config.get("budget_fallback", []):
            if name in self.config["providers"] and name not in order:
                order.append(name)
        return self.spill_saturated(order)

    def spill_saturated(self, order):
        """Move providers whose worker is at `max_in_flight` to the back.

        Two local GPU workers each serve a bounded number of requests well;
        past that, a request queues behind the others on the same card while
        the second card sits idle. A saturated provider is not removed — it is
        still the last resort — it just stops being first. Providers without
        `max_in_flight` are never moved, so cloud ordering is unchanged.
        """
        with self.in_flight_lock:
            busy = {name for name in order if self.saturated(name)}
        if not busy:
            return order
        return [name for name in order if name not in busy] + [name for name in order if name in busy]

    def claimed_attempts(self, chain):
        """Yield `attempt_order(chain)`, holding an in-flight slot on each.

        The slot is claimed in the same critical section that checks the
        limit, so two concurrent requests cannot both see an idle worker and
        pile onto it. It is held while the caller's loop body runs and
        released when the loop advances or exits (return, break, exception).
        """
        pending = self.attempt_order(chain)
        deferred = set()
        while pending:
            name = pending.pop(0)
            worker = self.worker_of(name)
            with self.in_flight_lock:
                if (name not in deferred and self.saturated(name)
                        and any(not self.saturated(other) for other in pending)):
                    deferred.add(name)
                    pending.append(name)
                    continue
                self.in_flight[worker] = self.in_flight.get(worker, 0) + 1
            try:
                yield name
            finally:
                with self.in_flight_lock:
                    self.in_flight[worker] = max(0, self.in_flight.get(worker, 0) - 1)

    def worker_of(self, name):
        """The load key: providers sharing a `worker` share one GPU's slots."""
        return self.config["providers"].get(name, {}).get("worker", name)

    def saturated(self, name):
        """True when `name`'s worker is at its `max_in_flight`. Caller holds the lock."""
        limit = self.config["providers"].get(name, {}).get("max_in_flight")
        return bool(limit) and self.in_flight.get(self.worker_of(name), 0) >= limit

    def track(self, name):
        """Context manager counting one running request against `name`."""
        router = self

        class _Tracker:
            def __enter__(self):
                with router.in_flight_lock:
                    worker = router.worker_of(name)
                    router.in_flight[worker] = router.in_flight.get(worker, 0) + 1

            def __exit__(self, *exc):
                with router.in_flight_lock:
                    worker = router.worker_of(name)
                    router.in_flight[worker] = max(0, router.in_flight.get(worker, 0) - 1)
                return False
        return _Tracker()

    def load(self):
        with self.in_flight_lock:
            return {name: count for name, count in self.in_flight.items() if count}

    def reserve(self, agent, estimate):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            total = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=?", (day,)).fetchone()[0]
            total += self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM reservations WHERE day=?", (day,)).fetchone()[0]
            spent = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=? AND agent=?", (day, agent)).fetchone()[0]
            spent += self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM reservations WHERE day=? AND agent=?", (day, agent)).fetchone()[0]
            if total + estimate > self.config["daily_budget_usd"] or spent + estimate > self.config["agent_daily_budget_usd"]:
                self.db.commit()
                return None
            reservation = uuid.uuid4().hex
            self.db.execute("INSERT INTO reservations VALUES (?,?,?,?,?)", (reservation, day, agent, estimate, time.time()))
            self.db.commit()
            return reservation

    def finish(self, reservation, agent, provider=None, model=None, usage=None, cost=0):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            self.db.execute("DELETE FROM reservations WHERE id=?", (reservation,))
            if provider is not None:
                self.db.execute("INSERT INTO usage (day,agent,provider,model,input_tokens,output_tokens,cost_usd,task_id) VALUES (?,?,?,?,?,?,?,?)", (day, agent, provider, model, usage.get("prompt_tokens", 0), usage.get("completion_tokens", 0), cost, getattr(self.context, "task_id", None)))
            self.db.commit()

    def stats(self):
        with self.lock:
            rows = self.db.execute("SELECT day,agent,provider,COUNT(*),ROUND(SUM(cost_usd),6) FROM usage GROUP BY day,agent,provider ORDER BY day DESC,agent").fetchall()
        return [{"day": d, "agent": a, "provider": p, "requests": n, "cost_usd": c} for d, a, p, n, c in rows]

    def reconcile_reservations(self, now=None):
        """Reclaim reservations left behind by a router that died mid-request.

        Nothing deletes a reservation except `finish`. If the process dies
        between reserving budget and calling the provider, the row is never
        removed: `reserved_usd` accumulates and the day's budget is consumed by
        requests that are not running. Startup calls this, so the next process
        to open the database starts from the truth.

        The TTL is far longer than any provider timeout, so a genuinely
        in-flight reservation is never reclaimed.
        """
        cutoff = (now if now is not None else time.time()) - self.config.get("reservation_ttl_seconds", 900)
        with self.lock:
            cursor = self.db.execute("DELETE FROM reservations WHERE created_at IS NULL OR created_at < ?", (cutoff,))
            self.db.commit()
            reclaimed = max(0, cursor.rowcount)
        self.reconciled_orphans += reclaimed
        return reclaimed

    def provider_cooldown(self, name, now=None):
        """Seconds this provider must be skipped for, or 0 when it is usable."""
        now = now if now is not None else time.time()
        with self.lock:
            row = self.db.execute("SELECT cooldown_until FROM provider_health WHERE provider=?", (name,)).fetchone()
        return max(0.0, (row[0] or 0) - now) if row else 0.0

    def note_provider_failure(self, name, error, retry_after=None):
        """Put a failing provider on cooldown so the next request skips it.

        Without this, a dead or rate-limited endpoint is retried by every
        request in turn: each one pays the timeout and the operators learn
        nothing until they read the logs. The delay doubles per consecutive
        failure and is capped, and an explicit `Retry-After` wins.
        """
        now = time.time()
        if retry_after is not None:
            try:
                delay = max(0.0, float(retry_after))
            except (TypeError, ValueError):
                delay = None
        else:
            delay = None
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            row = self.db.execute("SELECT failures FROM provider_health WHERE provider=?", (name,)).fetchone()
            failures = (row[0] or 0) + 1 if row else 1
            if delay is None:
                base = self.config.get("provider_cooldown_seconds", 60)
                delay = min(base * (2 ** min(failures - 1, 10)), self.config.get("provider_cooldown_max_seconds", 3600))
            self.db.execute(
                "INSERT INTO provider_health (provider,failures,cooldown_until,last_error,last_failure_at) VALUES (?,?,?,?,?) "
                "ON CONFLICT(provider) DO UPDATE SET failures=excluded.failures, cooldown_until=excluded.cooldown_until, "
                "last_error=excluded.last_error, last_failure_at=excluded.last_failure_at",
                (name, failures, now + delay, str(error)[:200], now))
            self.db.commit()
        return delay

    def note_provider_success(self, name):
        """A working provider starts its next request with a clean record."""
        with self.lock:
            self.db.execute("DELETE FROM provider_health WHERE provider=?", (name,))
            self.db.commit()

    def provider_health(self):
        now = time.time()
        with self.lock:
            rows = self.db.execute("SELECT provider,failures,cooldown_until,last_error FROM provider_health").fetchall()
        return [{"provider": p, "failures": f or 0, "cooldown_seconds": round(max(0.0, (u or 0) - now), 3), "last_error": e}
                for p, f, u, e in rows]

    def note_attempt(self, name, model, latency_ms, ok, retried=False, usage=None, cost=0.0):
        """Record one provider attempt, successful or not.

        Latency is taken from every attempt, including the ones that failed:
        a provider that is fast when it works and slow when it times out is a
        different routing proposition from one that is uniformly slow, and
        averaging only the successes hides exactly that.
        """
        usage = usage or {}
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            self.db.execute(
                "INSERT INTO provider_stats (provider,model,attempts,successes,failures,retries,"
                "latency_ms_total,latency_samples,input_tokens,output_tokens,cost_usd) "
                "VALUES (?,?,1,?,?,?,?,1,?,?,?) "
                "ON CONFLICT(provider,model) DO UPDATE SET "
                "attempts=attempts+1, successes=successes+excluded.successes, "
                "failures=failures+excluded.failures, retries=retries+excluded.retries, "
                "latency_ms_total=latency_ms_total+excluded.latency_ms_total, "
                "latency_samples=latency_samples+1, "
                "input_tokens=input_tokens+excluded.input_tokens, "
                "output_tokens=output_tokens+excluded.output_tokens, "
                "cost_usd=cost_usd+excluded.cost_usd",
                (name, model, 1 if ok else 0, 0 if ok else 1, 1 if retried else 0,
                 float(latency_ms), usage.get("prompt_tokens", 0) or 0,
                 usage.get("completion_tokens", 0) or 0, float(cost)))
            self.db.commit()

    def provider_registry(self):
        """Measured profiles, in the shape §3 asks for.

        `verified_patch_rate` is joined in from the task feedback table rather
        than invented here: it counts tasks whose recorded checks passed, which
        is the only evidence this router has that a patch was actually good.
        """
        with self.lock:
            rows = self.db.execute(
                "SELECT provider,model,attempts,successes,failures,retries,"
                "latency_ms_total,latency_samples,input_tokens,output_tokens,cost_usd "
                "FROM provider_stats ORDER BY provider,model").fetchall()
            outcomes = self.db.execute(
                "SELECT u.provider,u.model,"
                "COUNT(DISTINCT CASE WHEN t.outcome='checks_passed' THEN t.id END),"
                "COUNT(DISTINCT CASE WHEN t.outcome='checks_failed' THEN t.id END) "
                "FROM usage u JOIN tasks t ON t.id=u.task_id GROUP BY u.provider,u.model").fetchall()
        verified = {(p, m): (ok, bad) for p, m, ok, bad in outcomes}

        registry = []
        for (name, model, attempts, successes, failures, retries,
             latency_total, latency_samples, tokens_in, tokens_out, cost) in rows:
            ok, bad = verified.get((name, model), (0, 0))
            attempts = attempts or 0
            registry.append({
                "provider": name,
                "model": model,
                "attempts": attempts,
                "successes": successes or 0,
                "failures": failures or 0,
                "failure_rate": round((failures or 0) / attempts, 4) if attempts else None,
                "retries": retries or 0,
                "retry_rate": round((retries or 0) / attempts, 4) if attempts else None,
                "average_latency_ms": round((latency_total or 0) / latency_samples, 1) if latency_samples else None,
                "latency_samples": latency_samples or 0,
                "input_tokens": tokens_in or 0,
                "output_tokens": tokens_out or 0,
                "cost_usd": round(cost or 0, 6),
                "passed_tasks": ok or 0,
                "failed_tasks": bad or 0,
                "verified_patch_rate": round((ok or 0) / (ok + bad), 4) if (ok + bad) else None,
            })
        return registry

    def begin_task(self, task_id, agent, revision, scope):
        if not re.fullmatch(r"[A-Za-z0-9_-]{1,80}", task_id) or not re.fullmatch(r"[0-9a-f]{40}", revision) or scope != "router":
            raise ValueError("Expected task ID, 40-character revision, and router scope")
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            existing = self.db.execute("SELECT agent,revision,scope,outcome FROM tasks WHERE id=?", (task_id,)).fetchone()
            if existing and (existing[:3] != (agent, revision, scope) or existing[3] != "pending"):
                self.db.rollback()
                raise ValueError("Task binding differs or task is already finalized")
            self.db.execute("INSERT OR IGNORE INTO tasks (id,agent,revision,scope) VALUES (?,?,?,?)", (task_id, agent, revision, scope))
            self.db.execute("UPDATE tasks SET in_flight=in_flight+1 WHERE id=?", (task_id,))
            self.db.commit()
        self.context.task_id = task_id

    def end_task_request(self, elapsed_ms):
        task_id = getattr(self.context, "task_id", None)
        if task_id:
            with self.lock:
                self.db.execute("UPDATE tasks SET requests=requests+1,elapsed_ms=elapsed_ms+?,in_flight=in_flight-1 WHERE id=?", (elapsed_ms, task_id))
                self.db.commit()
        self.context.task_id = None

    def task_outcome(self, data):
        if not isinstance(data, dict):
            raise ValueError("Expected evidence object")
        checks = data.get("checks")
        if not isinstance(checks, list) or not checks or any(not isinstance(c, dict) or type(c.get("exit_code")) is not int or not re.fullmatch(r"[0-9a-f]{64}", c.get("output_sha256", "")) for c in checks):
            raise ValueError("Expected check exit codes and output SHA-256 digests")
        if data.get("scope") != "router" or [c.get("name") for c in checks] != ["router-tests"]:
            raise ValueError("Unsupported verification scope/checks")
        outcome = "checks_passed" if all(c["exit_code"] == 0 for c in checks) else "checks_failed"
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            task = self.db.execute("SELECT revision,scope,outcome,in_flight FROM tasks WHERE id=?", (data.get("task_id"),)).fetchone()
            if not task or task[:2] != (data.get("revision"), data.get("scope")) or task[2] != "pending" or task[3] != 0:
                self.db.rollback()
                raise ValueError("Unknown, mismatched, or finalized task")
            self.db.execute("UPDATE tasks SET outcome=?,evidence=? WHERE id=?", (outcome, json.dumps(checks), data["task_id"]))
            self.db.commit()
            attribution = self.db.execute(
                "SELECT provider,model FROM usage WHERE task_id=? ORDER BY input_tokens+output_tokens DESC LIMIT 1",
                (data["task_id"],)).fetchone()
        provider, model = attribution if attribution else (None, None)
        # Recorded after the outcome is committed: the verification result is
        # the durable fact, and a memory outage must not undo it.
        kind, recorded = self.record_outcome_memory(
            data["task_id"], data["scope"], data["revision"], outcome, checks, provider, model)
        return {"task_id": data["task_id"], "outcome": outcome, "scope": "router",
                "memory": {"kind": kind, "fingerprint": (recorded or {}).get("fingerprint"),
                           "recorded": recorded is not None}}

    def task_stats(self):
        with self.lock:
            rows = self.db.execute("SELECT t.id,t.revision,t.scope,t.requests,t.elapsed_ms,t.outcome,COALESCE(SUM(u.cost_usd),0) FROM tasks t LEFT JOIN usage u ON u.task_id=t.id GROUP BY t.id ORDER BY t.rowid DESC LIMIT 100").fetchall()
        return [dict(zip(("task_id", "revision", "scope", "requests", "elapsed_ms", "outcome", "cost_usd"), row)) for row in rows]

    def learning_stats(self):
        with self.lock:
            rows = self.db.execute("SELECT u.provider,u.model,t.scope,COUNT(DISTINCT CASE WHEN t.outcome='checks_passed' THEN t.id END),COUNT(DISTINCT CASE WHEN t.outcome='checks_failed' THEN t.id END),SUM(CASE WHEN t.outcome IN ('checks_passed','checks_failed') THEN u.cost_usd ELSE 0 END) FROM usage u JOIN tasks t ON t.id=u.task_id GROUP BY u.provider,u.model,t.scope").fetchall()
        return [{"provider": p, "model": m, "scope": s, "passed_tasks": ok, "failed_tasks": bad,
                 "finalized_cost_usd": cost, "cost_per_passed_task_usd": cost / ok if ok else None,
                 "pass_rate": ok / (ok + bad) if ok + bad else None}
                for p, m, s, ok, bad, cost in rows]

    def snapshot(self):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            spent, requests, inputs, outputs = self.db.execute(
                "SELECT COALESCE(SUM(cost_usd),0),COUNT(*),COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0) FROM usage WHERE day=?", (day,)
            ).fetchone()
            reserved, inflight = self.db.execute(
                "SELECT COALESCE(SUM(cost_usd),0),COUNT(*) FROM reservations WHERE day=?", (day,)
            ).fetchone()
            rows = self.db.execute(
                "SELECT agent,provider,COUNT(*),SUM(cost_usd) FROM usage WHERE day=? GROUP BY agent,provider ORDER BY SUM(cost_usd) DESC", (day,)
            ).fetchall()
        return {"day": day, "spent_usd": spent, "reserved_usd": reserved, "requests": requests,
                "inflight": inflight, "input_tokens": inputs, "output_tokens": outputs,
                "reconciled_orphans": self.reconciled_orphans,
                "daily_budget_usd": self.config["daily_budget_usd"],
                "breakdown": [{"agent": a, "provider": p, "requests": n, "cost_usd": c} for a, p, n, c in rows]}

    def diagnostic(self, provider, status, reason, attempt=None, message=None):
        """One sanitized refusal: who, what status, why, how long it is benched.

        `message` is the provider's own error text, already scrubbed of
        credential shapes by the caller. Without it a failure says "HTTP 400"
        and leaves the operator to guess which field was rejected; with it the
        attempt list names the field. Prompt text, headers and credential
        values never appear: the log line and the error body are read by
        operators and by failing clients, and neither should carry repository
        contents.
        """
        entry = {"provider": provider, "status": status, "reason": reason,
                 "cooldown_seconds": round(self.provider_cooldown(provider), 1),
                 "request_id": getattr(self.context, "request_id", None)}
        if message:
            entry["message"] = sanitize_secret(message)
        entry["attempt"] = attempt if attempt is not None else (
            provider + ": " + reason if provider else reason)
        if provider and message:
            entry["attempt"] = provider + ": " + reason + " message: " + entry["message"]
        self.log_diagnostic(entry)
        return entry

    def log_diagnostic(self, entry):
        print("x3-ai-router request_id=" + str(entry.get("request_id"))
              + " provider=" + str(entry.get("provider")) + " status=" + str(entry.get("status"))
              + " cooldown=" + str(entry.get("cooldown_seconds"))
              + " reason=" + str(entry.get("reason"))[:200]
              + (" message=" + str(entry["message"])[:200] if entry.get("message") else ""),
              file=sys.stderr, flush=True)

    def provider_skip(self, name, provider, tier, request):
        """Why this provider cannot take this request, or None to try it.

        An agent request carries tool schemas. A provider whose model cannot
        call tools answers in prose and the client waits for a tool call that
        can never arrive, so an agent request needs a provider that has
        declared `supports_tools` — text-only success is not success here.

        A provider that declares tool support but has been *observed* answering
        a tool request with prose is refused as well: configuration may grant
        the capability, only evidence may take it away.

        A provider the operator switched off is skipped quietly: it is a
        configuration choice, not a failure worth reporting on every request.
        """
        if provider.get("enabled_env") and os.environ.get(provider["enabled_env"]) != "1":
            return {"quiet": True}
        if tier == "critical" and not may_serve_critical(provider):
            return self.diagnostic(name, None, "not cleared for critical work")
        cooldown = self.provider_cooldown(name)
        if cooldown > 0:
            return self.diagnostic(name, None, f"cooling down for {cooldown:.0f}s")
        if request.get("tools") or request.get("functions"):
            if not provider.get(TOOL_CAPABLE, False):
                return self.diagnostic(name, None, "model is not declared tool-capable")
            verdict = self.probe_tools(name, provider)
            if verdict["tools"] is False:
                return self.diagnostic(name, None, "tool probe: " + verdict["detail"])
        error = pricing_error(provider)
        if error:
            return self.diagnostic(name, None, error)
        if provider.get("api_key_env") and not os.environ.get(provider["api_key_env"]):
            return self.diagnostic(name, None, "credential unavailable")
        return None

    def probe_tools(self, name, provider, now=None):
        """Does this endpoint really return a tool call, or only prose about one?

        `/api/show` reporting a `tools` capability was not enough: a local model
        was observed advertising tools and answering every tool request with
        `{"name": ..., "arguments": ...}` in the message body and `tool_calls`
        null. A declaration is a claim; this is the check.

        One sample is not a verdict. A free cloud model returned a genuine tool
        call on three consecutive startup probes and then answered the very next
        one with prose; sampled six times it managed four. The probe therefore
        takes several samples and reports the *rate*, and a provider counts as
        capable only if that rate clears `probe_min_success_rate`.

        The verdict is cached per provider and model, and a probe that cannot
        run leaves `tools` as None rather than revoking a working declaration.
        """
        key = (name, provider.get("model"))
        now = now if now is not None else time.time()
        if not provider.get("tool_probe"):
            return {"tools": None, "detail": "not probed", "checked_at": None,
                    "samples": 0, "genuine": 0, "success_rate": None}
        if PROTOCOL_CHAT not in provider_protocols(provider):
            # The probe is a Chat Completions request. A provider that does not
            # speak it cannot be measured this way, and a verdict invented for
            # it would be worse than no verdict.
            return {"tools": None, "detail": "not probed: provider does not speak Chat Completions",
                    "checked_at": None, "samples": 0, "genuine": 0, "success_rate": None}
        with self.lock:
            cached = self.capabilities.get(key)
        ttl = self.config.get("capability_probe_ttl_seconds", 3600)
        if cached and cached["checked_at"] is not None and now - cached["checked_at"] < ttl:
            return cached
        samples = max(1, int(provider.get("probe_samples", self.config.get("capability_probe_samples", 3))))
        threshold = float(provider.get("probe_min_success_rate", 1.0))
        genuine, attempted, failure = 0, 0, None
        try:
            headers = {"Content-Type": "application/json"}
            key_env = provider.get("api_key_env")
            if key_env and os.environ.get(key_env):
                headers["Authorization"] = "Bearer " + os.environ[key_env]
            # The probe is a required tool choice, so it has to obey the same
            # provider reasoning rules as a real forced-tool turn: DeepSeek
            # rejects `required` outright while thinking mode is on.
            probe_body = tool_probe_request(
                provider["model"],
                provider.get("tool_probe_max_tokens",
                             self.config.get("capability_probe_max_tokens", PROBE_MAX_TOKENS)))
            apply_provider_reasoning(probe_body, provider)
            payload = json.dumps(probe_body).encode()
            for _ in range(samples):
                call = urllib.request.Request(provider["base_url"].rstrip("/") + "/chat/completions",
                                              payload, headers, method="POST")
                with urllib.request.urlopen(call, timeout=provider.get("probe_timeout_seconds", 60)) as response:
                    result = json.load(response)
                message = (result.get("choices") or [{}])[0].get("message") or {}
                attempted += 1
                if any((c.get("function") or {}).get("name") == PROBE_TOOL
                       for c in (message.get("tool_calls") or [])):
                    genuine += 1
        except urllib.error.HTTPError as exc:
            failure = "probe failed: HTTP " + str(exc.code)
        except (urllib.error.URLError, TimeoutError, ValueError, OSError) as exc:
            failure = "probe failed: " + type(exc).__name__
        rate = (genuine / attempted) if attempted else None
        if failure is not None and attempted == 0:
            verdict = {"tools": None, "detail": failure, "checked_at": now,
                       "samples": samples, "genuine": genuine, "success_rate": None}
        else:
            reliable = rate is not None and rate >= threshold
            detail = ("returned a genuine tool call on {}/{} probes".format(genuine, attempted)
                      if reliable else
                      "answered a required tool request with text and no tool_calls array on "
                      "{}/{} probes".format(attempted - genuine, attempted))
            if failure is not None:
                detail += " (" + failure + ")"
            verdict = {"tools": reliable, "detail": detail, "checked_at": now,
                       "samples": samples, "genuine": genuine,
                       "success_rate": round(rate, 4) if rate is not None else None}
        with self.lock:
            self.capabilities[key] = verdict
        self.log_diagnostic({"request_id": "capability", "provider": name,
                             "status": verdict["tools"], "cooldown_seconds": 0,
                             "reason": "{model}: {genuine}/{attempted} genuine tool calls ({detail})".format(
                                 model=provider.get("model"), genuine=verdict["genuine"],
                                 attempted=attempted, detail=verdict["detail"])})
        return verdict

    def capability_report(self, probe=False):
        """What every configured provider claims, and what could be verified.

        This exists so a green `/health` is not mistaken for a working agent
        route: liveness and capability are different questions.
        """
        report = []
        for name, provider in self.config["providers"].items():
            verdict = (self.probe_tools(name, provider) if probe
                       else self.capabilities.get((name, provider.get("model")), {
                           "tools": None, "detail": "not probed", "checked_at": None}))
            report.append({
                "provider": name,
                "model": provider.get("model"),
                "protocols": list(provider_protocols(provider)),
                "hosted_tools": list(provider.get("hosted_tools") or ()),
                "declared_tools": bool(provider.get(TOOL_CAPABLE, False)),
                "probed_tools": verdict["tools"],
                "probe_detail": verdict["detail"],
                "probe_samples": verdict.get("samples", 0),
                "probe_genuine": verdict.get("genuine", 0),
                "probe_success_rate": verdict.get("success_rate"),
                "critical_allowed": may_serve_critical(provider),
                "credential_present": bool(not provider.get("api_key_env")
                                           or os.environ.get(provider["api_key_env"])),
            })
        return report

    def warm_capabilities(self):
        """Probe every opted-in provider once, without blocking startup."""
        for name, provider in self.config["providers"].items():
            if provider.get("tool_probe"):
                try:
                    self.probe_tools(name, provider)
                except Exception as exc:  # a probe must never take the router down
                    self.log_diagnostic({"request_id": "capability", "provider": name,
                                         "status": None, "cooldown_seconds": 0,
                                         "reason": "probe raised " + type(exc).__name__})

    def provider_payload(self, plan, provider, stream):
        """The upstream body for one provider attempt, in its own protocol.

        The client's `model` is always replaced: the router chooses. The output
        bound is always set, in whichever parameter the protocol names, so a
        request can never be billed for more than the reservation covers. A
        native Responses body is not otherwise rewritten; the provider's own
        fields (`instructions`, `input`, `reasoning`, `tool_choice`) ride along
        unchanged, which is the whole point of forwarding it natively.
        """
        plan = coerce_plan(plan)
        protocol = plan.protocol_for(provider)
        payload = dict(plan.shape(protocol) or {})
        payload["model"] = provider["model"]
        payload["stream"] = stream
        limit = output_bound(plan.control(), self.config)
        if protocol == PROTOCOL_RESPONSES:
            payload["max_output_tokens"] = limit
            normalize_responses_tools(payload, provider)
            return payload
        if stream:
            payload["stream_options"] = {"include_usage": True}
        payload["max_tokens"] = limit
        apply_provider_reasoning(payload, provider)
        if provider.get("output_token_parameter") == "max_completion_tokens":
            payload["max_completion_tokens"] = payload.pop("max_tokens", DEFAULT_OUTPUT_TOKENS)
        return payload

    def provider_url(self, provider, protocol):
        """The endpoint for the protocol, under the provider's base URL."""
        suffix = "/responses" if protocol == PROTOCOL_RESPONSES else "/chat/completions"
        return provider["base_url"].rstrip("/") + suffix

    def failure_body(self, diagnostics, kind="no_provider_succeeded", message="No provider succeeded"):
        return {"error": {"message": message, "type": kind,
                          "request_id": getattr(self.context, "request_id", None),
                          "attempts": [entry["attempt"] for entry in diagnostics],
                          "providers": [{key: value for key, value in entry.items()
                                         if key not in ("attempt", "quiet")} for entry in diagnostics]}}

    def retry_budget(self, provider):
        """Extra attempts beyond the first, for one provider.

        Bounded on purpose (§50): an unbounded retry turns a slow provider into
        a stalled request, and the fallback chain already exists to move on.
        """
        try:
            return max(0, int(provider.get("retry_attempts", self.config.get("retry_attempts", 1))))
        except (TypeError, ValueError):
            return 1

    def compile_context(self, query, budget=None, max_files=None):
        """Ask the Forge context compiler what a task needs to read (§7).

        The compiler is `tools/x3-forge/context.py`, run as a subprocess rather
        than reimplemented here: it already carries the provenance that makes a
        selection checkable, and a second copy would drift from it. The router's
        job is to make it reachable from the same place routing decisions are
        made, with the task text the classifier already extracted.

        Returns `(package, None)` or `(None, reason)`.
        """
        spec = self.config.get("context_compiler")
        if not spec:
            return None, "no context compiler is configured"
        command = list(spec.get("command") or [])
        if not command:
            return None, "context compiler command is empty"
        if budget is not None:
            command += ["--budget", str(int(budget))]
        if max_files is not None:
            command += ["--max-files", str(int(max_files))]
        command.append(query)
        try:
            completed = subprocess.run(
                command,
                cwd=spec.get("cwd") or None,
                capture_output=True,
                text=True,
                timeout=float(spec.get("timeout_seconds", 60)),
            )
        except subprocess.TimeoutExpired:
            return None, "context compiler timed out"
        except (OSError, ValueError) as exc:
            return None, "context compiler could not run: " + type(exc).__name__
        if completed.returncode != 0:
            detail = (completed.stderr or "").strip().splitlines()
            hint = detail[-1] if detail else "no output"
            return None, "context compiler exited " + str(completed.returncode) + ": " + hint[:200]
        try:
            return json.loads(completed.stdout), None
        except ValueError:
            return None, "context compiler did not return JSON"

    def compile_context_once(self, query, budget=None, max_files=None):
        """As above, and returns the wall-clock milliseconds it took."""
        started = time.monotonic()
        package, error = self.compile_context(query, budget, max_files)
        return package, error, self.elapsed_ms(started)

    def memory_command(self, extra):
        """Build the `failure_memory.py` invocation, or None if unconfigured."""
        spec = self.config.get("failure_memory")
        if not spec or not spec.get("command"):
            return None, None
        return list(spec["command"]) + extra, spec

    def run_memory(self, extra, timeout_default=30):
        """Run the memory tool. Returns `(parsed_json, reason)`.

        Memory is advisory: an outage must not fail a verification submission,
        so every caller here logs the reason and carries on.
        """
        command, spec = self.memory_command(extra)
        if command is None:
            return None, "no failure memory is configured"
        try:
            completed = subprocess.run(
                command, cwd=spec.get("cwd") or None, capture_output=True, text=True,
                timeout=float(spec.get("timeout_seconds", timeout_default)))
        except subprocess.TimeoutExpired:
            return None, "failure memory timed out"
        except (OSError, ValueError) as exc:
            return None, "failure memory could not run: " + type(exc).__name__
        if completed.returncode != 0:
            detail = (completed.stderr or "").strip().splitlines()
            return None, "failure memory exited " + str(completed.returncode) + ": " + (
                detail[-1] if detail else "no output")[:200]
        try:
            return json.loads(completed.stdout), None
        except ValueError:
            return None, "failure memory did not return JSON"

    def search_memory(self, query, kind=None, component=None, limit=10):
        extra = ["find", query, "--json", "--limit", str(int(limit))]
        if kind:
            extra += ["--kind", kind]
        if component:
            extra += ["--component", component]
        return self.run_memory(extra)

    def record_outcome_memory(self, task_id, scope, revision, outcome, checks, provider=None, model=None):
        """Feed one task outcome into the Forge memories (§17, §18).

        A failed task becomes a failure entry carrying the failing check and
        the provider that produced it; a passed task becomes a success entry
        (§18: "also record what worked"). Both are advisory writes — the
        submission result is already committed by the time this runs, and a
        memory outage is reported, not raised.
        """
        failing = [check for check in checks if check.get("exit_code") != 0]
        if outcome == "checks_failed":
            kind = "failure"
            names = ", ".join(str(check.get("name")) for check in failing) or "unnamed check"
            error = "task checks failed: " + names
            extra = ["add", "--kind", "failure", "--component", scope,
                     "--error", error, "--trigger", str(task_id), "--commit", revision]
        else:
            kind = "success"
            names = ", ".join(str(check.get("name")) for check in checks) or "no checks"
            extra = ["add", "--kind", "success", "--component", scope,
                     "--error", "task checks passed: " + names,
                     "--trigger", str(task_id), "--commit", revision,
                     "--fix", "checks " + names + " passed on this revision"]
        if provider:
            extra += ["--model", str(provider) + ("/" + model if model else "")]
        extra += ["--json"]
        recorded, reason = self.run_memory(extra)
        if reason:
            print("x3-ai-router memory write skipped: " + reason, file=sys.stderr, flush=True)
        return kind, recorded

    def retry_delay(self, provider, index):
        """Exponential backoff, capped. Retry-After still wins where sent."""
        try:
            base = float(provider.get("retry_backoff_ms", self.config.get("retry_backoff_ms", 250))) / 1000.0
            cap = float(self.config.get("retry_backoff_max_ms", 2_000)) / 1000.0
        except (TypeError, ValueError):
            return 0.25
        return max(0.0, min(cap, base * (2 ** max(0, index))))

    def elapsed_ms(self, started):
        return (time.monotonic() - started) * 1000.0

    def complete(self, plan, agent):
        plan = coerce_plan(plan)
        request = plan.control()
        # UTF-8 JSON bytes conservatively bound visible input tokens; reject
        # oversized requests instead of trusting a configured estimate.
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config.get("max_request_bytes", 8 * 1024 * 1024):
            return 413, {"error": {"message": "Request body exceeds configured byte limit", "type": "request_body_too_large"}}
        error = request_error(request, self.config)
        if error:
            return 400, {"error": {"message": error}}
        tier, chain, classification, logical = self.choose(request)
        self.context.routing = {"policy": logical, "tier": tier,
                                "task_class": classification["task_class"]}
        failures = []
        budget_refused = False
        for name in self.claimed_attempts(chain):
            provider = self.config["providers"][name]
            model = provider["model"]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            protocol = plan.protocol_for(provider)
            skip = self.provider_skip(name, provider, tier, request)
            if skip is None and protocol is None:
                # The provider declared a protocol this surface cannot use. A
                # clean named refusal beats sending a body it will reject.
                skip = self.diagnostic(name, None, plan.refusal(provider))
            if skip is not None:
                if not skip.get("quiet"):
                    failures.append(skip)
                continue
            # Reserve against an upper-bound configured for each request before making the call.
            estimate = (output_bound(request, self.config) * price_out + self.config["max_input_tokens"] * price_in) / 1_000_000
            key = os.environ.get(provider.get("api_key_env", ""), "") if provider.get("api_key_env") else ""
            # A provider that cannot bill needs no reservation, and must not be
            # blocked by a budget that is already spent. Refusing a free local
            # model because a paid API is over its ceiling turns "stop spending"
            # into "stop working".
            if estimate <= 0:
                reservation = None
            else:
                reservation = self.reserve(agent, estimate)
                if reservation is None:
                    budget_refused = True
                    failures.append(self.diagnostic(name, None, "daily budget exhausted"))
                    continue
            payload = self.provider_payload(plan, provider, False)
            headers = {"Content-Type": "application/json"}
            if key:
                headers["Authorization"] = "Bearer " + key
            url = self.provider_url(provider, protocol)
            body = json.dumps(payload).encode()
            allowed = 1 + self.retry_budget(provider)
            for attempt_index in range(allowed):
                started = time.monotonic()
                try:
                    call = urllib.request.Request(url, body, headers, method="POST")
                    with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                        result = json.load(response)
                    if protocol == PROTOCOL_RESPONSES:
                        if not is_responses_object(result):
                            raise ValueError("Provider response is not a Responses object")
                        if result.get("status") == "failed":
                            # HTTP 200 with a failed body is the Responses
                            # protocol's way of reporting an upstream failure.
                            # Booking it as a success would hand the client an
                            # empty answer and leave a broken provider marked
                            # healthy, so it fails over like any other error.
                            error = result.get("error") if isinstance(result.get("error"), dict) else {}
                            detail = "response.failed"
                            if error.get("message"):
                                detail += ": " + sanitize_secret(str(error["message"]))[:160]
                            self.note_attempt(name, model, self.elapsed_ms(started), False, attempt_index > 0)
                            self.finish(reservation, agent)
                            self.note_provider_failure(name, detail)
                            failures.append(self.diagnostic(name, None, detail))
                            break
                    else:
                        choices = result.get("choices") if isinstance(result, dict) else None
                        if (not isinstance(choices, list) or not choices
                                or any(not isinstance(choice, dict)
                                       or not valid_completion_message(choice.get("message"))
                                       for choice in choices)):
                            raise ValueError("Provider response has invalid completion choices")
                    usage = normalize_usage(result.get("usage", {}))
                    cost = (usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000 if usage else estimate
                    self.note_attempt(name, model, self.elapsed_ms(started), True, attempt_index > 0, usage, cost)
                    self.note_provider_success(name)
                    self.finish(reservation, agent, name, model, usage, cost)
                    return 200, result
                except urllib.error.HTTPError as exc:
                    # HTTPError is a subclass of URLError, so it has to be
                    # caught first to read a rate-limit `Retry-After`.
                    self.note_attempt(name, model, self.elapsed_ms(started), False, attempt_index > 0)
                    retry_after = exc.headers.get("Retry-After") if exc.headers else None
                    detail = http_error_detail(exc)
                    if exc.code in RETRYABLE_STATUS and attempt_index + 1 < allowed:
                        time.sleep(self.retry_delay(provider, attempt_index))
                        continue
                    self.finish(reservation, agent)
                    self.note_provider_failure(
                        name, "HTTP " + str(exc.code) + (": " + detail if detail else ""), retry_after)
                    failures.append(self.diagnostic(name, exc.code, "HTTP " + str(exc.code), message=detail))
                    break
                except (urllib.error.URLError, TimeoutError) as exc:
                    self.note_attempt(name, model, self.elapsed_ms(started), False, attempt_index > 0)
                    if attempt_index + 1 < allowed:
                        time.sleep(self.retry_delay(provider, attempt_index))
                        continue
                    self.finish(reservation, agent)
                    self.note_provider_failure(name, type(exc).__name__)
                    failures.append(self.diagnostic(name, None, type(exc).__name__))
                    break
                except ValueError as exc:
                    # A malformed body is not transient: the same request will
                    # produce the same malformed answer.
                    self.note_attempt(name, model, self.elapsed_ms(started), False, attempt_index > 0)
                    self.finish(reservation, agent)
                    self.note_provider_failure(name, type(exc).__name__)
                    failures.append(self.diagnostic(name, None, type(exc).__name__))
                    break
                except Exception:
                    self.note_attempt(name, model, self.elapsed_ms(started), False, attempt_index > 0)
                    self.finish(reservation, agent)
                    self.note_provider_failure(name, "unexpected error")
                    raise
        if budget_refused:
            return 429, self.failure_body(failures, "budget_exceeded", "Daily budget exhausted")
        return 502, self.failure_body(failures)

    def relay_chat_stream(self, response, started, start, send):
        """Relay a Chat Completions SSE stream; return (usage, reported_failure).

        `[DONE]` or a finish reason is the only evidence the answer finished.
        Reading past `[DONE]` to wait for the socket to close hung the handler
        on a keep-alive connection the provider never closed.
        """
        usage = None
        finish_reason = None
        saw_done = False
        for line in response:
            if len(line) > 1_000_000:
                raise ValueError("Oversized SSE line")
            if not line.startswith(b"data: "):
                if started:
                    send(line)
                continue
            data = line[6:].strip()
            if not started:
                start()
                started.append(True)
            if data == b"[DONE]":
                saw_done = True
                send(line)
                break
            event = json.loads(data)
            if event.get("usage"):
                usage = event["usage"]
            for choice in event.get("choices") or []:
                if choice.get("finish_reason"):
                    finish_reason = choice["finish_reason"]
            send(line)
        if not started:
            raise ValueError("Empty SSE response")
        if not saw_done and finish_reason is None:
            raise ValueError("Stream ended without a terminal event")
        return normalize_usage(usage), None

    def relay_responses_stream(self, response, started, start, send):
        """Relay a native Responses SSE stream; return (usage, reported_failure).

        The stream terminates with a semantic event, so `[DONE]` is never the
        end of it. The first event is held until it is known not to be a
        failure: a provider that fails before producing any output can then be
        failed over without the client seeing a half-started stream.
        """
        reader = ResponsesEventReader()
        held = []
        for line in response:
            if len(line) > 1_000_000:
                raise ValueError("Oversized SSE line")
            stripped = line.rstrip(b"\r\n")
            if not started:
                held.append(line)
                if not reader.begins(stripped):
                    continue
                if reader.first_event_type(stripped) == "response.failed":
                    raise ValueError("provider reported response.failed before any output")
                start()
                started.append(True)
                for buffered in held:
                    send(buffered)
                held = []
                reader.read(stripped)
                if reader.stops(stripped):
                    break
                continue
            reader.read(stripped)
            send(line)
            if reader.stops(stripped):
                break
        if not started:
            raise ValueError("Empty SSE response")
        if reader.terminal_reason() is not None:
            raise ValueError("Stream ended without a terminal event")
        reported = "provider reported " + str(reader.status) if reader.failed else None
        return normalize_usage(reader.usage), reported

    def stream(self, plan, agent, start, send):
        plan = coerce_plan(plan)
        request = plan.control()
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config.get("max_request_bytes", 8 * 1024 * 1024):
            return 413, {"error": "Request body exceeds configured byte limit"}
        error = request_error(request, self.config)
        if error:
            return 400, {"error": error}
        tier, chain, classification, logical = self.choose(request)
        self.context.routing = {"policy": logical, "tier": tier,
                                "task_class": classification["task_class"]}
        failures = []
        budget_refused = False
        for name in self.claimed_attempts(chain):
            provider = self.config["providers"][name]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            protocol = plan.protocol_for(provider)
            skip = self.provider_skip(name, provider, tier, request)
            if skip is None and protocol is None:
                skip = self.diagnostic(name, None, plan.refusal(provider))
            if skip is not None:
                if not skip.get("quiet"):
                    failures.append(skip)
                continue
            payload = self.provider_payload(plan, provider, True)
            key = os.environ.get(provider.get("api_key_env", ""), "") if provider.get("api_key_env") else ""
            estimate = (output_bound(request, self.config) * price_out + self.config["max_input_tokens"] * price_in) / 1_000_000
            # A provider that cannot bill needs no reservation, and must not be
            # blocked by a budget that is already spent. Refusing a free local
            # model because a paid API is over its ceiling turns "stop spending"
            # into "stop working".
            if estimate <= 0:
                reservation = None
            else:
                reservation = self.reserve(agent, estimate)
                if reservation is None:
                    budget_refused = True
                    failures.append(self.diagnostic(name, None, "daily budget exhausted"))
                    continue
            headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
            if key:
                headers["Authorization"] = "Bearer " + key
            body = json.dumps(payload).encode()
            allowed = 1 + self.retry_budget(provider)
            for attempt_index in range(allowed):
                attempt_started = time.monotonic()
                # Reset per attempt: a retry is only legal before any byte has
                # reached the client, and a partial stream must never be
                # stitched onto a fresh one.
                started = []
                usage = None
                try:
                    def begin():
                        # Tell the caller's callback which protocol won, so a
                        # Chat Completions stream is re-emitted as Responses
                        # events while a native one is relayed verbatim.
                        self.context.stream_protocol = protocol
                        start()

                    call = urllib.request.Request(self.provider_url(provider, protocol),
                                                  body, headers, method="POST")
                    with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                        if "text/event-stream" not in response.headers.get("Content-Type", ""):
                            raise ValueError("Provider did not return SSE")
                        if protocol == PROTOCOL_RESPONSES:
                            usage, reported = self.relay_responses_stream(response, started, begin, send)
                        else:
                            usage, reported = self.relay_chat_stream(response, started, begin, send)
                    if reported:
                        # The provider reported its own failure once output had
                        # begun. It is relayed to the client, benched in provider
                        # health, and never booked as a success. What it already
                        # produced is what it bills, so the usage it reported is
                        # what is charged, not the reservation estimate.
                        cost = ((usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000) if usage else estimate
                        self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started),
                                          False, attempt_index > 0, usage or {}, cost)
                        self.note_provider_failure(name, reported)
                        self.finish(reservation, agent, name, provider["model"], usage or {}, cost)
                        return None
                    cost = ((usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000) if usage else estimate
                    self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started), True,
                                      attempt_index > 0, usage or {}, cost)
                    self.note_provider_success(name)
                    self.finish(reservation, agent, name, provider["model"], usage or {}, cost)
                    return None
                except ClientDisconnected:
                    # The caller hung up. Charge what the provider already
                    # produced, release the reservation, and say nothing: there
                    # is no socket left to answer on and no traceback worth
                    # printing.
                    self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started), False,
                                      attempt_index > 0)
                    self.finish(reservation, agent, name if started else None, provider["model"],
                                usage or {}, estimate if started else 0)
                    self.diagnostic(name, None, "client disconnected")
                    return None
                except urllib.error.HTTPError as exc:
                    retry_after = exc.headers.get("Retry-After") if exc.headers else None
                    detail = http_error_detail(exc)
                    self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started), False,
                                      attempt_index > 0)
                    # A 5xx or a 429 before the first byte is worth one more
                    # try; after the first byte it is not, because the client
                    # already has half an answer.
                    if not started and exc.code in RETRYABLE_STATUS and attempt_index + 1 < allowed:
                        time.sleep(self.retry_delay(provider, attempt_index))
                        continue
                    self.note_provider_failure(
                        name, "HTTP " + str(exc.code) + (": " + detail if detail else ""), retry_after)
                    failure = self.diagnostic(name, exc.code, "HTTP " + str(exc.code), message=detail)
                    if started:
                        self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                        return 502, self.failure_body([failure])
                    self.finish(reservation, agent)
                    failures.append(failure)
                    break
                except (urllib.error.URLError, TimeoutError, OSError) as exc:
                    self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started), False,
                                      attempt_index > 0)
                    if not started and attempt_index + 1 < allowed:
                        time.sleep(self.retry_delay(provider, attempt_index))
                        continue
                    self.note_provider_failure(name, type(exc).__name__)
                    failure = self.diagnostic(name, None, type(exc).__name__)
                    if started:
                        self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                        return 502, self.failure_body([failure])
                    self.finish(reservation, agent)
                    failures.append(failure)
                    break
                except ValueError as exc:
                    # A malformed or truncated body is not transient: repeating
                    # the identical request produces the identical answer.
                    self.note_attempt(name, provider["model"], self.elapsed_ms(attempt_started), False,
                                      attempt_index > 0)
                    self.note_provider_failure(name, str(exc)[:200])
                    failure = self.diagnostic(name, None, sanitize_secret(str(exc)))
                    if started:
                        self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                        return 502, self.failure_body([failure])
                    self.finish(reservation, agent)
                    failures.append(failure)
                    break
                except Exception:
                    self.finish(reservation, agent, name if started else None, provider["model"], usage or {}, estimate if started else 0)
                    self.note_provider_failure(name, "unexpected error")
                    raise
        if budget_refused:
            return 429, self.failure_body(failures, "budget_exceeded", "Daily budget exhausted")
        return 502, self.failure_body(failures)


def dashboard(snapshot):
    rows = "".join("<tr>" + "".join(f"<td>{html.escape(str(item[key]))}</td>" for key in ("agent", "provider", "requests", "cost_usd")) + "</tr>"
                   for item in snapshot["breakdown"])
    cells = "".join(f"<li><strong>{html.escape(key.replace('_', ' ').title())}:</strong> {html.escape(str(value))}</li>"
                    for key, value in snapshot.items() if key != "breakdown")
    return ("<!doctype html><html lang='en'><meta charset='utf-8'><meta http-equiv='refresh' content='15'>"
            "<meta name='viewport' content='width=device-width,initial-scale=1'><title>X3 AI router</title>"
            "<style>body{font:16px system-ui;background:#111827;color:#f9fafb;max-width:960px;margin:3rem auto;padding:1rem}"
            "table{border-collapse:collapse;width:100%}td,th{padding:.7rem;border-bottom:1px solid #4b5563;text-align:left}"
            "li{margin:.5rem 0}a{color:#fb923c}</style><h1>X3 AI router</h1><ul>" + cells +
            "</ul><h2>Today by agent and provider</h2><table><thead><tr><th>Agent</th><th>Provider</th>"
            "<th>Requests</th><th>USD</th></tr></thead><tbody>" + rows + "</tbody></table></html>")


def metrics(snapshot):
    fields = ("spent_usd", "reserved_usd", "requests", "inflight", "input_tokens", "output_tokens",
              "daily_budget_usd", "reconciled_orphans")
    return "".join(f"x3_ai_router_{name} {snapshot[name]}\n" for name in fields)


def handler_for(router):
    class Handler(BaseHTTPRequestHandler):
        # Tool names already reported as dropped. Codex sends a disabled
        # `web_search` on every turn; saying so once is a record, saying it
        # every turn is noise that buries the real diagnostics.
        reported_disabled = set()

        def handle_one_request(self):
            # A client that hangs up mid-answer is ordinary, not an incident.
            # Without this the base server prints a traceback per disconnect.
            try:
                super().handle_one_request()
            except (BrokenPipeError, ConnectionResetError):
                self.close_connection = True

        def reply(self, status, data):
            if self.wfile is None:
                return
            body = json.dumps(data).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def authorized(self):
            secret = os.environ.get("X3_ROUTER_TOKEN")
            auth = self.headers.get("Authorization", "")
            return not secret or auth in ("Bearer " + secret, "Basic " + base64.b64encode(("x3:" + secret).encode()).decode())

        def raw(self, status, body, content_type):
            data = body.encode()
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            if not self.authorized():
                self.send_response(401)
                self.send_header("WWW-Authenticate", 'Basic realm="X3 AI router"')
                self.end_headers()
                return
            if self.path == "/health":
                return self.reply(200, {"status": "ok", "in_flight": router.load()})
            if self.path == "/v1/usage":
                return self.reply(200, {"usage": router.stats()})
            if self.path == "/v1/tasks":
                return self.reply(200, {"tasks": router.task_stats()})
            if self.path == "/v1/learning":
                return self.reply(200, {"routing_mode": "fixed", "models": router.learning_stats()})
            if self.path == "/v1/dashboard":
                return self.raw(200, dashboard(router.snapshot()), "text/html; charset=utf-8")
            if self.path == "/metrics":
                return self.raw(200, metrics(router.snapshot()), "text/plain; version=0.0.4; charset=utf-8")
            if self.path == "/v1/models":
                # The logical models are routing policies, not model names.
                policies = router.config.get("policies") or DEFAULT_POLICIES
                data = [{"id": name, "object": "model",
                         "description": "routing policy: " + " -> ".join(policy.get("order", []))}
                        for name, policy in policies.items()]
                return self.reply(200, {"object": "list", "data": data})
            if self.path.startswith("/v1/models/"):
                wanted = self.path.rsplit("/", 1)[-1]
                policies = router.config.get("policies") or DEFAULT_POLICIES
                if wanted in policies:
                    return self.reply(200, {"id": wanted, "object": "model"})
                return self.reply(404, {"error": {"message": "No such model"}})
            if self.path == "/v1/registry":
                return self.reply(200, {"registry": router.provider_registry()})
            context_path, _, context_query = self.path.partition("?")
            if context_path == "/v1/context":
                # Same intent as /v1/explain, for the other half of the
                # question: what should the model read?
                query = ""
                for pair in context_query.split("&"):
                    key, _, value = pair.partition("=")
                    if key == "q":
                        query = urllib.parse.unquote_plus(value)
                if not query:
                    return self.reply(400, {"error": {"message": "Expected ?q=<task>",
                                                      "type": "missing_query"}})
                package, error, elapsed = router.compile_context_once(query)
                if error:
                    return self.reply(502, {"error": {"message": error,
                                                      "type": "context_unavailable"}})
                return self.reply(200, {"package": package, "elapsed_ms": round(elapsed, 1)})
            memory_path, _, memory_query = self.path.partition("?")
            if memory_path == "/v1/memory":
                # §17: "Before debugging: SEARCH FAILURE MEMORY."
                params = {}
                for pair in memory_query.split("&"):
                    key, _, value = pair.partition("=")
                    if key:
                        params[key] = urllib.parse.unquote_plus(value)
                query = params.get("q", "")
                if not query:
                    return self.reply(400, {"error": {"message": "Expected ?q=<task>",
                                                      "type": "missing_query"}})
                try:
                    limit = max(1, min(50, int(params.get("limit", "10"))))
                except ValueError:
                    return self.reply(400, {"error": {"message": "limit must be an integer"}})
                matches, reason = router.search_memory(
                    query, params.get("kind"), params.get("component"), limit)
                if reason:
                    return self.reply(502, {"error": {"message": reason,
                                                      "type": "memory_unavailable"}})
                return self.reply(200, {"query": query, "matches": matches})
            if self.path == "/v1/providers":
                return self.reply(200, {"providers": router.provider_health()})
            capabilities, _, query = self.path.partition("?")
            if capabilities == "/v1/capabilities":
                # `?probe=1` re-checks every provider that opted in; the plain
                # form reports the cached verdicts so a monitor can poll it.
                return self.reply(200, {"capabilities": router.capability_report(probe="probe=1" in query)})
            return self.reply(404, {"error": "Not found"})

        def stream_chunk(self, chunk):
            try:
                self.wfile.write(chunk)
                self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError, OSError) as exc:
                raise ClientDisconnected() from exc

        def serve_responses(self, data, agent):
            """Serve `POST /v1/responses` in whichever protocol the provider speaks.

            A provider that declares the Responses protocol is handed the
            request natively — only `model`, the output bound and any tool the
            provider cannot run are touched — so Codex and DeepSeek speak the
            same protocol end to end. A provider that only speaks Chat
            Completions still gets the tested translation, which is what keeps
            it in the failover chain. The two shapes are built once and the
            provider decides which one it gets, so provider selection, budgets,
            cooldowns and fallback are not duplicated per protocol.
            """
            if not isinstance(data.get("input"), (list, str)) or not data.get("input"):
                return self.reply(400, {"error": {"message": "Expected an input list"}})
            if not isinstance(data.get("stream", False), bool):
                # An omitted `stream` means a JSON envelope: required by the
                # Responses API contract, and the Codex client only switches
                # to SSE when it explicitly asks for it.
                return self.reply(400, {"error": {"message": "stream must be a boolean"}})
            if isinstance(data["input"], str):
                data = dict(data, input=responses_input_items(data))
            chat, custom, disabled, chat_error = None, set(), [], None
            try:
                chat, custom, disabled = responses_request(data)
            except UnsupportedFeature as exc:
                # A hosted tool with no Chat Completions equivalent only rules
                # out the Chat providers. It is not fatal while a provider that
                # speaks Responses natively can still take the request.
                chat_error = str(exc)
            if chat is not None and not chat["messages"]:
                return self.reply(400, {"error": {"message": "Expected at least one input message"}})
            if chat is None and not any(
                    PROTOCOL_RESPONSES in provider_protocols(router.config["providers"][name])
                    for name in router.attempt_order(router.choose(data)[1])):
                # Nothing in the chain can express this request. Fail closed,
                # naming the capability rather than sending a malformed body.
                return self.reply(400, {"error": {
                    "message": "Unsupported feature for the available providers: " + str(chat_error),
                    "type": "unsupported_feature",
                    "unsupported": str(chat_error)}})
            error = request_error(data, router.config)
            if error:
                return self.reply(400, {"error": {"message": error}})
            for name in disabled:
                if name not in Handler.reported_disabled:
                    Handler.reported_disabled.add(name)
                    # Named, never silent: an operator can see that a tool the
                    # client sent with the request disabled was not offered to
                    # the model.
                    self.log_message("tool disabled by the client, not offered to the model: %s", name)

            plan = UpstreamRequest(PROTOCOL_RESPONSES, data, chat=chat, responses=data,
                                   custom=custom, disabled=disabled, chat_error=chat_error)
            response_id = "resp_" + uuid.uuid4().hex
            model = plan.model()

            if not data.get("stream", False):
                status, result = router.complete(plan, agent)
                if status != 200:
                    return self.reply(status, result)
                if is_responses_object(result):
                    # A native Responses provider already answered in the exact
                    # shape the client speaks; re-wrapping it would only lose
                    # fields (reasoning items, output indices, status).
                    return self.reply(200, result)
                message = (result.get("choices") or [{}])[0].get("message") or {}
                envelope = responses_envelope(response_id, result.get("model", model),
                                              chat_message_to_response_output(message, response_id, custom),
                                              result.get("usage"))
                return self.reply(200, envelope)

            started = []
            adapter = []

            def start():
                started.append(True)
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Cache-Control", "no-cache")
                self.send_header("Connection", "close")
                self.end_headers()
                if getattr(router.context, "stream_protocol", None) != PROTOCOL_RESPONSES:
                    # A Chat Completions provider answers a Responses client
                    # through the adapter, which rebuilds the event sequence
                    # from its chunks.
                    adapter.append(ResponsesStream(self.stream_chunk, response_id, model, custom))
                    adapter[0].start()

            def send(chunk):
                if adapter:
                    adapter[0].on_line(chunk)
                else:
                    self.stream_chunk(chunk)

            outcome = router.stream(plan, agent, start, send)
            try:
                if adapter:
                    if outcome is not None and adapter[0].started:
                        # The response already began, so the only honest ending
                        # is a failure event rather than a status code the
                        # client can no longer see.
                        adapter[0].fail(outcome[1].get("error", {}).get("message", "upstream failure"),
                                        status=outcome[0])
                    elif outcome is not None:
                        return self.reply(*outcome)
                    else:
                        adapter[0].finish()
                elif outcome is not None:
                    if started:
                        self.stream_chunk(
                            b"event: response.failed\ndata: "
                            + json.dumps(responses_failure_event(
                                response_id, model,
                                outcome[1].get("error", {}).get("message", "upstream failure"),
                                status=outcome[0])).encode() + b"\n\n")
                    else:
                        return self.reply(*outcome)
            except ClientDisconnected:
                pass
            self.close_connection = True
            return

        def do_POST(self):
            if self.path == "/v1/tasks/outcome":
                token = os.environ.get("X3_VERIFIER_TOKEN")
                if not token or token == os.environ.get("X3_ROUTER_TOKEN") or self.headers.get("Authorization") != "Bearer " + token:
                    return self.reply(403, {"error": "Independent verifier authorization required"})
                try:
                    size = int(self.headers.get("Content-Length", "0"))
                    if not 0 < size <= 65536:
                        return self.reply(413, {"error": "Invalid evidence size"})
                    return self.reply(200, router.task_outcome(json.loads(self.rfile.read(size))))
                except (ValueError, TypeError, KeyError):
                    return self.reply(400, {"error": "Invalid verification evidence"})
            if not self.authorized():
                return self.reply(401, {"error": "Unauthorized"})
            if self.path == "/v1/explain":
                # Answers "what would you do with this, and why" without
                # calling a provider. Routing a request is a decision worth
                # being able to audit on its own, and it costs nothing to ask.
                try:
                    size = int(self.headers.get("Content-Length", "0"))
                    if size < 1 or size > MAX_BODY:
                        return self.reply(413, {"error": "Invalid request size"})
                    data = json.loads(self.rfile.read(size))
                    if isinstance(data.get("input"), list):
                        chat, custom, disabled = responses_request(data)
                        # Classify the request as the client wrote it: a
                        # Responses request carries its signal in `input` and
                        # `instructions`, not in the translated messages.
                        classified = data
                    else:
                        chat, custom, disabled = data, set(), []
                        classified = chat
                    tier, chain, classification, logical = router.choose(classified)
                except UnsupportedFeature as exc:
                    return self.reply(400, {"error": {"message": str(exc),
                                                      "type": "unsupported_feature"}})
                except (ValueError, TypeError, KeyError):
                    return self.reply(400, {"error": "Invalid request"})
                # `context: true` also answers the §7 half of the question.
                # Opt-in, because compiling a package is a subprocess reading a
                # large index and nobody should pay for it by accident.
                context = None
                if data.get("context") is True:
                    package, error, elapsed = router.compile_context_once(task_text(chat))
                    context = {"error": error, "elapsed_ms": round(elapsed, 1)} if error else \
                              {"package": package, "elapsed_ms": round(elapsed, 1)}
                # §17: search the memories before the work starts, so a known
                # problem is not rediscovered. Opt-in for the same reason as
                # context: it is a subprocess call.
                memory = None
                if data.get("memory") is True:
                    matches, reason = router.search_memory(task_text(chat), limit=5)
                    memory = {"error": reason} if reason else {"matches": matches}
                return self.reply(200, {
                    "policy": logical,
                    "tier": tier,
                    "provider_order": router.attempt_order(chain),
                    "classification": classification,
                    "disabled_tools": disabled,
                    "context": context,
                    "memory": memory,
                })
            if any(self.path == path or self.path.startswith(path + "/") for path in UNSUPPORTED_PATHS):
                return self.reply(501, {"error": {"message": self.path + " is not implemented: this router speaks the Chat Completions API at /v1/chat/completions"}})
            if self.path not in ("/v1/chat/completions", "/v1/responses"):
                return self.reply(404, {"error": "Not found"})
            try:
                started = time.monotonic()
                size = int(self.headers.get("Content-Length", "0"))
                if size < 1 or size > MAX_BODY:
                    return self.reply(413, {"error": "Invalid request size"})
                data = json.loads(self.rfile.read(size))
                agent = self.headers.get("X-X3-Agent", "default")[:80]
                # One id per request, echoed in failures and in the sanitized
                # operator log so a 502 can be traced without log diving.
                router.context.request_id = uuid.uuid4().hex
                task_id = self.headers.get("X-X3-Task-ID")
                if task_id:
                    router.begin_task(task_id, agent, self.headers.get("X-X3-Revision", ""), self.headers.get("X-X3-Scope", "router"))
                if self.path == "/v1/responses":
                    return self.serve_responses(data, agent)
                if not isinstance(data.get("messages"), list) or not isinstance(data.get("stream", False), bool):
                    return self.reply(400, {"error": "Expected messages and boolean stream"})
                error = request_error(data, router.config)
                if error:
                    return self.reply(400, {"error": error})
                if data.get("stream"):
                    started_stream = []

                    def start():
                        started_stream.append(True)
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.send_header("Connection", "close")
                        self.end_headers()

                    def send(chunk):
                        try:
                            self.wfile.write(chunk)
                            self.wfile.flush()
                        except (BrokenPipeError, ConnectionResetError, OSError) as exc:
                            raise ClientDisconnected() from exc

                    try:
                        outcome = router.stream(UpstreamRequest(PROTOCOL_CHAT, data, chat=data),
                                                agent, start, send)
                    except ClientDisconnected:
                        self.close_connection = True
                        return
                    if outcome is not None and not started_stream:
                        return self.reply(*outcome)
                    self.close_connection = True
                    return
                status, result = router.complete(UpstreamRequest(PROTOCOL_CHAT, data, chat=data), agent)
                return self.reply(status, result)
            except (ValueError, TypeError, KeyError):
                return self.reply(400, {"error": "Invalid request"})
            finally:
                router.end_task_request((time.monotonic() - started) * 1000)
    return Handler


REGISTRATION_KEYS = {"base_url", "protocol", "model", "supports_tools", "tool_probe", "tool_probe_max_tokens",
                     "probe_timeout_seconds", "probe_samples", "critical_allowed", "max_in_flight", "worker",
                     "timeout_seconds"}


def apply_registrations(config, directory):
    """Merge GPU worker drop-ins from `directory` into `config`.

    A node joins the fabric by writing `<node>.json` here instead of editing
    `config.json`:

        {"node": "x3gpu2",
         "providers": {"x3gpu2_gpu0_qwen3": {"base_url": "http://x3gpu2:11434/v1",
                                             "model": "qwen3:8b", "worker": "x3gpu2-gpu0", ...}},
         "policies": {"x3-local": ["x3gpu2_gpu0_qwen3"], "x3-code": ["x3gpu2_gpu0_qwen3"]}}

    Registered providers are appended to the named policies (after what the
    operator configured, so a new node never jumps the queue) and to
    `budget_fallback`. They are credential-free and price-free by
    construction: only keys in REGISTRATION_KEYS are accepted, so a drop-in
    cannot smuggle in an API key, a price, or `free_model`. A bad file is
    skipped with a message rather than stopping the router.
    """
    accepted, rejected = [], []
    if not directory or not os.path.isdir(directory):
        return accepted, rejected
    for filename in sorted(os.listdir(directory)):
        if not filename.endswith(".json"):
            continue
        path = os.path.join(directory, filename)
        try:
            with open(path, encoding="utf-8") as source:
                data = json.load(source)
            providers = data.get("providers")
            if not isinstance(providers, dict) or not providers:
                raise ValueError("no providers")
            for name, spec in providers.items():
                if name in config["providers"]:
                    raise ValueError(f"provider {name!r} already exists")
                if not isinstance(spec, dict) or set(spec) - REGISTRATION_KEYS:
                    raise ValueError(f"provider {name!r} has unsupported keys {sorted(set(spec) - REGISTRATION_KEYS)}")
                if not str(spec.get("base_url", "")).startswith(("http://", "https://")) or not spec.get("model"):
                    raise ValueError(f"provider {name!r} needs an http(s) base_url and a model")
            policies = data.get("policies") or {}
            for policy, names in policies.items():
                if policy not in config.get("policies", {}):
                    raise ValueError(f"unknown policy {policy!r}")
                if not set(names) <= set(providers):
                    raise ValueError(f"policy {policy!r} names providers outside this file")
        except (OSError, ValueError, TypeError, AttributeError) as exc:
            rejected.append({"file": filename, "error": str(exc)})
            continue
        for name, spec in providers.items():
            config["providers"][name] = dict({"protocol": "chat_completions", "critical_allowed": False}, **spec)
        for policy, names in policies.items():
            config["policies"][policy]["order"] = config["policies"][policy].get("order", []) + list(names)
        fallback = config.setdefault("budget_fallback", [])
        fallback.extend(name for name in providers if name not in fallback)
        accepted.append({"file": filename, "node": data.get("node"), "providers": sorted(providers)})
    return accepted, rejected


def default_db_path():
    """`$X3_ROUTER_DB`, else `$XDG_DATA_HOME/x3-router/usage.sqlite3`.

    The old default was relative to the working directory, so the same
    service started from two places kept two separate budgets.
    """
    if os.environ.get("X3_ROUTER_DB"):
        return os.environ["X3_ROUTER_DB"]
    base = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"), ".local", "share")
    return os.path.join(base, "x3-router", "usage.sqlite3")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default=os.path.join(os.path.dirname(__file__), "config.json"))
    parser.add_argument("--db", default=default_db_path())
    parser.add_argument("--port", type=int, default=11435)
    parser.add_argument("--providers-dir", default=os.environ.get(
        "X3_ROUTER_PROVIDERS_DIR", os.path.join(os.path.expanduser("~"), ".config", "x3-router", "providers.d")))
    # Loopback by default. Set a LAN address only together with
    # X3_ROUTER_TOKEN; the router fronts paid providers.
    parser.add_argument("--host", default=os.environ.get("X3_ROUTER_HOST", "127.0.0.1"))
    args = parser.parse_args()
    with open(args.config, encoding="utf-8") as source:
        config = json.load(source)
    accepted, rejected = apply_registrations(config, args.providers_dir)
    for entry in accepted:
        print("registered worker", json.dumps(entry), flush=True)
    for entry in rejected:
        print("rejected worker registration", json.dumps(entry), flush=True)
    router = Router(config, args.db)
    # Probe in the background: a slow or unreachable provider must not hold up
    # the listener, and the verdict is what makes an agent route provable.
    threading.Thread(target=router.warm_capabilities, daemon=True).start()
    if args.host not in ("127.0.0.1", "localhost", "::1") and not os.environ.get("X3_ROUTER_TOKEN"):
        parser.error("--host other than loopback requires X3_ROUTER_TOKEN")
    server = ThreadingHTTPServer((args.host, args.port), handler_for(router))
    server.serve_forever()


if __name__ == "__main__":
    main()
