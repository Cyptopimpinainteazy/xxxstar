#!/usr/bin/env python3
"""Verify patched Solana pubsub over TLS against an owned, disposable validator."""
import argparse
import asyncio
import contextlib
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import ssl
import subprocess
import tempfile
import time
import urllib.request


def available_base():
    for _ in range(100):
        with socket.socket() as candidate:
            candidate.bind(("127.0.0.1", 0))
            base = candidate.getsockname()[1]
        if base > 65000:
            continue
        with contextlib.ExitStack() as stack:
            try:
                for port in range(base, base + 42):
                    for kind in (socket.SOCK_STREAM, socket.SOCK_DGRAM):
                        sock = stack.enter_context(socket.socket(type=kind))
                        sock.bind(("127.0.0.1", port))
            except OSError:
                continue
            return base
    raise RuntimeError("no free loopback port range")


def rpc(port, method):
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method}).encode(),
        headers={"Content-Type": "application/json"},
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(request, timeout=2) as response:
        data = json.load(response)
    if "error" in data:
        raise RuntimeError(str(data["error"]))
    return data["result"]


async def pipe(reader, writer):
    while data := await reader.read(65536):
        writer.write(data)
        await writer.drain()


async def run(probe, evidence):
    validator = shutil.which("solana-test-validator")
    openssl = shutil.which("openssl")
    if not validator or not openssl or not probe.is_file():
        raise RuntimeError("validator, OpenSSL and prebuilt probe are required")
    summary = {"status": "RUNNING", "scope": "ephemeral local Solana slot subscription over TLS"}
    summary.update(started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                   probe_sha256=hashlib.sha256(probe.read_bytes()).hexdigest(),
                   script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    repository = Path(__file__).resolve().parents[2]
    summary["repository_head"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repository, text=True).strip()
    summary["repository_dirty"] = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=repository, text=True))
    evidence.mkdir(parents=True, exist_ok=True)
    validator_process = None
    with tempfile.TemporaryDirectory(prefix="x3-solana-tls-") as directory:
        work = Path(directory)
        try:
            base = available_base()
            certificate = work / "certificate.pem"
            key = work / "key.pem"
            subprocess.run([
                openssl, "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", str(key), "-out", str(certificate), "-subj", "/CN=localhost",
                "-addext", "subjectAltName=DNS:localhost", "-days", "1",
            ], check=True, capture_output=True, timeout=15)
            os.chmod(key, 0o600)
            version = subprocess.run([validator, "--version"], check=True, capture_output=True,
                                     text=True, timeout=10).stdout.strip()
            summary["validator_version"] = version
            command = [validator, "--quiet", "--ledger", str(work / "ledger"),
                       "--bind-address", "127.0.0.1", "--rpc-port", str(base),
                       "--faucet-port", str(base + 2), "--gossip-port", str(base + 3),
                       "--dynamic-port-range", f"{base + 10}-{base + 41}",
                       "--limit-ledger-size", "10000"]
            summary["validator_command"] = command
            with (work / "validator.log").open("w") as log:
                validator_process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT,
                    env={**os.environ, "SOLANA_METRICS_CONFIG": ""}, start_new_session=True)
                summary["validator_pid"] = validator_process.pid
                deadline = time.monotonic() + 90
                while True:
                    if validator_process.poll() is not None:
                        raise RuntimeError(f"owned validator exited {validator_process.returncode}")
                    try:
                        if rpc(base, "getHealth") == "ok":
                            break
                    except (OSError, RuntimeError, ValueError):
                        pass
                    if time.monotonic() >= deadline:
                        raise RuntimeError("owned validator RPC did not become healthy")
                    await asyncio.sleep(0.5)
                summary["rpc_start_slot"] = rpc(base, "getSlot")
                context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                context.minimum_version = ssl.TLSVersion.TLSv1_2
                context.load_cert_chain(certificate, key)

                async def tunnel(reader, writer):
                    upstream_writer = None
                    tasks = []
                    try:
                        upstream_reader, upstream_writer = await asyncio.open_connection("127.0.0.1", base + 1)
                        tasks = [asyncio.create_task(pipe(reader, upstream_writer)),
                                 asyncio.create_task(pipe(upstream_reader, writer))]
                        await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
                    finally:
                        for task in tasks:
                            task.cancel()
                        await asyncio.gather(*tasks, return_exceptions=True)
                        for stream in (writer, upstream_writer):
                            if stream is not None:
                                stream.close()
                                with contextlib.suppress(Exception):
                                    await asyncio.wait_for(stream.wait_closed(), 2)

                async with await asyncio.start_server(tunnel, "127.0.0.1", 0, ssl=context) as server:
                    port = server.sockets[0].getsockname()[1]
                    endpoint = f"wss://localhost:{port}/"
                    trust_dir = work / "empty-trust-directory"
                    trust_dir.mkdir()
                    child = await asyncio.create_subprocess_exec(str(probe), endpoint,
                        env={**os.environ, "SSL_CERT_FILE": str(certificate),
                             "SSL_CERT_DIR": str(trust_dir)},
                        stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
                    try:
                        stdout, stderr = await asyncio.wait_for(child.communicate(), 30)
                    except BaseException:
                        with contextlib.suppress(ProcessLookupError):
                            child.kill()
                        await child.wait()
                        raise
                    summary.update(probe_exit_code=child.returncode, probe_stdout=stdout.decode(),
                                   probe_stderr=stderr.decode())
                    if child.returncode != 0 or not stdout.decode().startswith("slot="):
                        raise RuntimeError("real Solana TLS subscription probe failed")
                summary["status"] = "CHECKS_PASS_CLEANUP_PENDING"
        except BaseException as error:
            summary.update(status="FAIL", error=str(error))
            raise
        finally:
            if validator_process is not None and validator_process.poll() is None:
                with contextlib.suppress(ProcessLookupError):
                    os.killpg(validator_process.pid, signal.SIGINT)
                try:
                    validator_process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    with contextlib.suppress(ProcessLookupError):
                        os.killpg(validator_process.pid, signal.SIGKILL)
                    validator_process.wait(timeout=10)
            summary["completed_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
            summary["owned_validator_stopped"] = validator_process is None or validator_process.poll() is not None
            log = work / "validator.log"
            if log.exists():
                (evidence / "validator-tail.log").write_text("\n".join(log.read_text(errors="replace").splitlines()[-250:]))
            summary["temporary_ledger_and_keys_cleanup"] = "TemporaryDirectory removes only this invocation's files"
            (evidence / "result.json").write_text(json.dumps(summary, indent=2) + "\n")
    summary["temporary_cleanup_verified"] = not work.exists()
    summary["status"] = "PASS" if summary["temporary_cleanup_verified"] else "FAIL"
    (evidence / "result.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    if summary["status"] != "PASS":
        raise RuntimeError("owned temporary ledger cleanup failed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    asyncio.run(run(args.probe.resolve(), args.evidence.resolve()))


if __name__ == "__main__":
    main()
