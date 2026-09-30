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
import sys
import threading
import time
import uuid
import urllib.error
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
# The single Chat Completions argument that carries a custom (freeform) tool's
# body. Chat Completions has no freeform tool type, so the text travels as one
# string argument and is lifted back out on the way to the client.
CUSTOM_TOOL_INPUT = "input"
TOOL_CAPABLE = "supports_tools"
# A model can advertise tool support and still answer with JSON in the message
# body. The only evidence that counts is a `tool_calls` array, so the probe asks
# for one and will not take prose for an answer.
PROBE_TOOL = "x3_capability_probe"
PROBE_ARGUMENT = "value"


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


def tool_probe_request(model):
    """A request whose only purpose is to be answered with a tool call."""
    return {
        "model": model, "stream": False, "max_tokens": 64, "tool_choice": "required",
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
    for item in request.get("input") or []:
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


def chat_message_to_response_output(message, prefix, custom=frozenset()):
    """Chat Completions message -> the Responses `output` list."""
    output = []
    content = message.get("content")
    if content:
        output.append({"id": prefix + "msg", "type": "message", "role": "assistant", "status": "completed",
                       "content": [{"type": "output_text", "text": content, "annotations": []}]})
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
        prompt = usage.get("prompt_tokens", 0) or 0
        completion = usage.get("completion_tokens", 0) or 0
        envelope["usage"] = {"input_tokens": prompt, "output_tokens": completion,
                             "total_tokens": usage.get("total_tokens", prompt + completion)}
    return envelope


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
    parameter the client actually set. Reading only `max_tokens` let a request
    that set `max_completion_tokens` instead be reserved at the 4096 default
    while the provider billed for whatever it asked for.
    """
    for key in ("max_tokens", "max_completion_tokens"):
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
    for key in ("max_tokens", "max_completion_tokens"):
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


class Router:
    def __init__(self, config, db_path):
        self.config = config
        self.db = sqlite3.connect(db_path, check_same_thread=False)
        self.lock = threading.Lock()
        self.context = threading.local()
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
        self.db.commit()
        self.reconciled_orphans = 0
        self.capabilities = {}
        self.reconcile_reservations()

    def choose(self, request):
        text = " ".join(str(m.get("content", "")) for m in request.get("messages", [])).lower()
        tier = "critical" if any(term in text for term in CRITICAL) else "routine"
        return tier, self.config["routes"][tier]

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
        return order

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
        return {"task_id": data["task_id"], "outcome": outcome, "scope": "router"}

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

    def diagnostic(self, provider, status, reason, attempt=None):
        """One sanitized refusal: who, what status, why, how long it is benched.

        Deliberately no prompt text, no headers and no credential values: the
        log line and the error body are read by operators and by failing
        clients, and neither should carry repository contents.
        """
        entry = {"provider": provider, "status": status, "reason": reason,
                 "cooldown_seconds": round(self.provider_cooldown(provider), 1),
                 "request_id": getattr(self.context, "request_id", None)}
        entry["attempt"] = attempt if attempt is not None else (
            provider + ": " + reason if provider else reason)
        self.log_diagnostic(entry)
        return entry

    def log_diagnostic(self, entry):
        print("x3-ai-router request_id=" + str(entry.get("request_id"))
              + " provider=" + str(entry.get("provider")) + " status=" + str(entry.get("status"))
              + " cooldown=" + str(entry.get("cooldown_seconds"))
              + " reason=" + str(entry.get("reason"))[:200], file=sys.stderr, flush=True)

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
            probe_body = tool_probe_request(provider["model"])
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

    def provider_payload(self, request, provider, stream):
        """The upstream Chat Completions body for one provider attempt.

        The client's `model` is ignored: the router chooses. `max_tokens` is
        always set, even when the client left it out, so the request can never
        be billed for more than the reservation covers.
        """
        payload = dict(request)
        payload["model"] = provider["model"]
        payload["stream"] = stream
        if stream:
            payload["stream_options"] = {"include_usage": True}
        payload["max_tokens"] = output_bound(request, self.config)
        apply_provider_reasoning(payload, provider)
        if provider.get("output_token_parameter") == "max_completion_tokens":
            payload["max_completion_tokens"] = payload.pop("max_tokens", DEFAULT_OUTPUT_TOKENS)
        return payload

    def failure_body(self, diagnostics, kind="no_provider_succeeded", message="No provider succeeded"):
        return {"error": {"message": message, "type": kind,
                          "request_id": getattr(self.context, "request_id", None),
                          "attempts": [entry["attempt"] for entry in diagnostics],
                          "providers": [{key: value for key, value in entry.items()
                                         if key not in ("attempt", "quiet")} for entry in diagnostics]}}

    def complete(self, request, agent):
        # UTF-8 JSON bytes conservatively bound visible input tokens; reject
        # oversized requests instead of trusting a configured estimate.
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config.get("max_request_bytes", 8 * 1024 * 1024):
            return 413, {"error": {"message": "Request body exceeds configured byte limit", "type": "request_body_too_large"}}
        error = request_error(request, self.config)
        if error:
            return 400, {"error": {"message": error}}
        tier, chain = self.choose(request)
        failures = []
        budget_refused = False
        for name in self.attempt_order(chain):
            provider = self.config["providers"][name]
            model = provider["model"]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            skip = self.provider_skip(name, provider, tier, request)
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
            payload = self.provider_payload(request, provider, False)
            headers = {"Content-Type": "application/json"}
            if key:
                headers["Authorization"] = "Bearer " + key
            url = provider["base_url"].rstrip("/") + "/chat/completions"
            try:
                call = urllib.request.Request(url, json.dumps(payload).encode(), headers, method="POST")
                with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                    result = json.load(response)
                if not isinstance(result, dict) or "choices" not in result:
                    raise ValueError("Provider response lacks choices")
                usage = result.get("usage", {})
                cost = (usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000 if usage else estimate
                self.note_provider_success(name)
                self.finish(reservation, agent, name, model, usage, cost)
                return 200, result
            except urllib.error.HTTPError as exc:
                # HTTPError is a subclass of URLError, so it has to be caught
                # first to read a rate-limit `Retry-After` instead of guessing.
                self.finish(reservation, agent)
                retry_after = exc.headers.get("Retry-After") if exc.headers else None
                self.note_provider_failure(name, "HTTP " + str(exc.code), retry_after)
                failures.append(self.diagnostic(name, exc.code, "HTTP " + str(exc.code)))
            except (urllib.error.URLError, TimeoutError, ValueError) as exc:
                self.finish(reservation, agent)
                self.note_provider_failure(name, type(exc).__name__)
                failures.append(self.diagnostic(name, None, type(exc).__name__))
            except Exception:
                self.finish(reservation, agent)
                self.note_provider_failure(name, "unexpected error")
                raise
        if budget_refused:
            return 429, self.failure_body(failures, "budget_exceeded", "Daily budget exhausted")
        return 502, self.failure_body(failures)

    def stream(self, request, agent, start, send):
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config.get("max_request_bytes", 8 * 1024 * 1024):
            return 413, {"error": "Request body exceeds configured byte limit"}
        error = request_error(request, self.config)
        if error:
            return 400, {"error": error}
        tier, chain = self.choose(request)
        failures = []
        budget_refused = False
        for name in self.attempt_order(chain):
            provider = self.config["providers"][name]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            skip = self.provider_skip(name, provider, tier, request)
            if skip is not None:
                if not skip.get("quiet"):
                    failures.append(skip)
                continue
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
            payload = self.provider_payload(request, provider, True)
            headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
            if key:
                headers["Authorization"] = "Bearer " + key
            emitted = False
            usage = None
            saw_done = False
            finish_reason = None
            try:
                call = urllib.request.Request(provider["base_url"].rstrip("/") + "/chat/completions",
                                              json.dumps(payload).encode(), headers, method="POST")
                with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                    if "text/event-stream" not in response.headers.get("Content-Type", ""):
                        raise ValueError("Provider did not return SSE")
                    for line in response:
                        if len(line) > 1_000_000:
                            raise ValueError("Oversized SSE line")
                        if not line.startswith(b"data: "):
                            if emitted:
                                send(line)
                            continue
                        data = line[6:].strip()
                        if not emitted:
                            start()
                            emitted = True
                        if data == b"[DONE]":
                            # The stream is over. Reading until the socket
                            # closes instead left the client waiting on a
                            # keep-alive connection the provider never closed.
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
                if not emitted:
                    raise ValueError("Empty SSE response")
                if not saw_done and finish_reason is None:
                    raise ValueError("Stream ended without a terminal event")
                cost = ((usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000) if usage else estimate
                self.note_provider_success(name)
                self.finish(reservation, agent, name, provider["model"], usage or {}, cost)
                return None
            except ClientDisconnected:
                # The caller hung up. Charge what the provider already
                # produced, release the reservation, and say nothing: there is
                # no socket left to answer on and no traceback worth printing.
                self.finish(reservation, agent, name if emitted else None, provider["model"],
                            usage or {}, estimate if emitted else 0)
                self.diagnostic(name, None, "client disconnected")
                return None
            except urllib.error.HTTPError as exc:
                retry_after = exc.headers.get("Retry-After") if exc.headers else None
                self.note_provider_failure(name, "HTTP " + str(exc.code), retry_after)
                failure = self.diagnostic(name, exc.code, "HTTP " + str(exc.code))
                if emitted:
                    self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                    # The client already has half an answer. It gets a terminal
                    # failure event, not a `response.completed`.
                    return 502, self.failure_body([failure])
                self.finish(reservation, agent)
                failures.append(failure)
            except (urllib.error.URLError, TimeoutError, ValueError, OSError) as exc:
                self.note_provider_failure(name, type(exc).__name__)
                failure = self.diagnostic(name, None, type(exc).__name__)
                if emitted:
                    self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                    # A partial stream cannot be retried with another model, so
                    # the client is told it failed rather than handed a
                    # truncated answer dressed up as a complete one.
                    return 502, self.failure_body([failure])
                self.finish(reservation, agent)
                failures.append(failure)
            except Exception:
                self.finish(reservation, agent, name if emitted else None, provider["model"], usage or {}, estimate if emitted else 0)
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
                return self.reply(200, {"status": "ok"})
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
                return self.reply(200, {"object": "list", "data": [{"id": "x3-auto", "object": "model"}]})
            if self.path.startswith("/v1/models/"):
                if self.path.rsplit("/", 1)[-1] == "x3-auto":
                    return self.reply(200, {"id": "x3-auto", "object": "model"})
                return self.reply(404, {"error": {"message": "No such model"}})
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
            """Serve `POST /v1/responses` by translating onto the chat path.

            Codex accepts only the Responses wire protocol for a custom
            provider, while every provider this router talks to speaks Chat
            Completions. Rather than duplicate provider selection, budgets,
            cooldowns and fallback, the request is translated and handed to
            `router.complete` / `router.stream`, and the answer is translated
            back.
            """
            if not isinstance(data.get("input"), list):
                return self.reply(400, {"error": {"message": "Expected an input list"}})
            try:
                chat, custom, disabled = responses_request(data)
            except UnsupportedFeature as exc:
                # Fail closed rather than silently dropping a capability the
                # caller believed it had. The client can then pick a provider
                # that speaks the Responses protocol natively.
                return self.reply(400, {"error": {
                    "message": "Unsupported feature for Chat Completions providers: " + str(exc),
                    "type": "unsupported_feature",
                    "unsupported": str(exc)}})
            if not chat["messages"]:
                return self.reply(400, {"error": {"message": "Expected at least one input message"}})
            error = request_error(chat, router.config)
            if error:
                return self.reply(400, {"error": {"message": error}})
            for name in disabled:
                if name not in Handler.reported_disabled:
                    Handler.reported_disabled.add(name)
                    # Named, never silent: an operator can see that a tool the
                    # client sent with the request disabled was not offered to
                    # the model.
                    self.log_message("tool disabled by the client, not offered to the model: %s", name)

            response_id = "resp_" + uuid.uuid4().hex
            model = data.get("model") if isinstance(data.get("model"), str) else "x3-auto"

            if not data.get("stream", True):
                status, result = router.complete(chat, agent)
                if status != 200:
                    return self.reply(status, result)
                message = (result.get("choices") or [{}])[0].get("message") or {}
                envelope = responses_envelope(response_id, result.get("model", model),
                                              chat_message_to_response_output(message, response_id, custom),
                                              result.get("usage"))
                return self.reply(200, envelope)

            stream = ResponsesStream(self.stream_chunk, response_id, model, custom)

            def start():
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Cache-Control", "no-cache")
                self.send_header("Connection", "close")
                self.end_headers()
                stream.start()

            outcome = router.stream(chat, agent, start, stream.on_line)
            try:
                if outcome is not None and stream.started:
                    # The response already began, so the only honest ending is
                    # a failure event rather than a status code the client
                    # cannot see any more.
                    stream.fail(outcome[1].get("error", {}).get("message", "upstream failure"),
                                status=outcome[0])
                elif outcome is not None:
                    return self.reply(*outcome)
                else:
                    stream.finish()
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
                        outcome = router.stream(data, agent, start, send)
                    except ClientDisconnected:
                        self.close_connection = True
                        return
                    if outcome is not None and not started_stream:
                        return self.reply(*outcome)
                    self.close_connection = True
                    return
                status, result = router.complete(data, agent)
                return self.reply(status, result)
            except (ValueError, TypeError, KeyError):
                return self.reply(400, {"error": "Invalid request"})
            finally:
                router.end_task_request((time.monotonic() - started) * 1000)
    return Handler


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default=os.path.join(os.path.dirname(__file__), "config.json"))
    parser.add_argument("--db", default="x3-router.sqlite3")
    parser.add_argument("--port", type=int, default=11435)
    args = parser.parse_args()
    with open(args.config, encoding="utf-8") as source:
        config = json.load(source)
    router = Router(config, args.db)
    # Probe in the background: a slow or unreachable provider must not hold up
    # the listener, and the verdict is what makes an agent route provable.
    threading.Thread(target=router.warm_capabilities, daemon=True).start()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler_for(router))
    server.serve_forever()


if __name__ == "__main__":
    main()
