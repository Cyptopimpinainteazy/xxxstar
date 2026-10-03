"""X3 cluster jobs: one command, at one exact commit, on one chosen worker, with evidence.

A job is routed by class to the role that owns that work (inventory.json), runs in a
throwaway detached worktree of the exact commit on the worker, and leaves a record in
audit-artifacts/x3-cluster/<commit>/jobs/<id>/:

    job.json        id, class, priority, requirements, worker, label, commit, toolchain,
                    kernel, start/end, exit code, result, log hash, artifact hashes
    log.txt         full combined output
    artifacts/      everything the command wrote to $X3_JOB_OUT (fuzz crashes, sim bundles)

Labels: PHYSICAL (ran on another machine over the LAN), LOCAL (ran here because this
node owns the role), LOCAL-FALLBACK (the owning role has no usable node, so the control
node ran it; see "BUILD SERVER FAILURE"). Classes with no fallback (GPU, INFERENCE) are
REJECTED instead of silently running somewhere that cannot do the work.
"""
import datetime as dt
import hashlib
import io
import json
import re
import secrets
import shlex
import shutil
import socket
import subprocess
import tarfile
import threading
import time
from pathlib import Path

# class -> owning role, role that may stand in for it, minimum free disk (GB) on the worker.
CLASSES = {
    "BUILD": {"role": "build", "fallback": "control", "min_free_gb": 30},
    "TEST": {"role": "build", "fallback": "control", "min_free_gb": 30},
    "FUZZ": {"role": "sim", "fallback": "control", "min_free_gb": 30},
    "SIMULATION": {"role": "sim", "fallback": "control", "min_free_gb": 20},
    "GPU": {"role": "gpu", "fallback": None, "min_free_gb": 8},
    "INFERENCE": {"role": "gpu", "fallback": None, "min_free_gb": 1},
    "NETWORK": {"role": "net", "fallback": "control", "min_free_gb": 5},
    "DATABASE": {"role": "data", "fallback": "control", "min_free_gb": 5},
    "INTEGRATION": {"role": "net", "fallback": "control", "min_free_gb": 20},
}
PRIORITIES = {"CRITICAL": 0, "HIGH": 5, "NORMAL": 10, "BACKGROUND": 19}  # -> nice level
# Exit codes the job wrapper reserves for "never ran the command".
REJECT_CODES = {96: "worker has too little free disk", 97: "commit not available on worker",
                98: "could not create worktree"}

PROBE = r"""python3 - <<'PY'
import json, os, shutil
from pathlib import Path
env = {}
p = Path.home() / ".config/x3-cluster/node.env"
if p.exists():
    for line in p.read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("#"):
            k, _, v = line.partition("=")
            env[k.strip()] = v.strip().strip('"')
print(json.dumps({"hostname": os.uname().nodename, "threads": os.cpu_count(), "load": os.getloadavg()[0],
                  "free_gb": round(shutil.disk_usage(str(Path.home())).free / 1e9, 1), "env": env}))
PY"""

DEFAULT_REPOS = ("~/Desktop/xxxstar-main", "~/Desktop/xxxstar-master", "~/xxxstar")
# Jobs fetch commits from here, never from whatever a worker's own `origin` points at.
CANONICAL_REMOTE = "https://github.com/Cyptopimpinainteazy/xxxstar.git"


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def ssh_cmd(host, connect_timeout=5):
    return ["ssh", "-o", "BatchMode=yes", "-o", f"ConnectTimeout={connect_timeout}", host]


def probe(name, node, me):
    """Facts a scheduler needs about one node, or None if it cannot be used."""
    if name == me:
        cmd = ["bash", "-c", PROBE]
    elif node.get("ip"):
        cmd = ssh_cmd(name if name_resolves(name) else node["ip"]) + [PROBE]
    else:
        return None
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=20)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if out.returncode != 0:
        return None
    try:
        facts = json.loads(out.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError):
        return None
    if name != me and facts.get("hostname") != name:
        # Something answered at that address, but it is not the node we think it is.
        return None
    return facts


def name_resolves(name):
    try:
        return not socket.gethostbyname(name).startswith("127.")
    except OSError:
        return False


def choose_worker(cls, inv, me, probe_fn=probe, pin=None, min_free_gb=None):
    """(node name, facts, label, rejected) for a job of class `cls`.

    Prefers the least-loaded usable node of the owning role; falls back to the
    class's stand-in role; returns name None when nothing qualifies."""
    spec = CLASSES[cls]
    need = spec["min_free_gb"] if min_free_gb is None else min_free_gb
    rejected = []

    def usable(names):
        found = []
        for name in names:
            facts = probe_fn(name, inv["nodes"][name], me)
            if facts is None:
                rejected.append({"node": name, "reason": "unreachable or SSH key auth failed"})
            elif facts["free_gb"] < need:
                rejected.append({"node": name, "reason": f"{facts['free_gb']} GB free < {need} GB required"})
            else:
                found.append((facts["load"] / max(1, facts["threads"]), name, facts))
        return sorted(found, key=lambda f: (f[0], f[1]))

    if pin:
        if pin not in inv["nodes"]:
            return None, None, None, [{"node": pin, "reason": "not in inventory.json"}]
        names, roles = [pin], [inv["nodes"][pin]["role"]]
    else:
        roles = [spec["role"]] + ([spec["fallback"]] if spec["fallback"] else [])
        names = None
    for role in roles:
        candidates = names or [n for n, node in inv["nodes"].items() if node["role"] == role]
        found = usable(candidates)
        if found:
            _, name, facts = found[0]
            label = "LOCAL" if name == me else "PHYSICAL"
            if role != spec["role"] and not pin:
                label += "-FALLBACK"
            return name, facts, label, rejected
    return None, None, None, rejected


def worker_repo(facts):
    """The node's checkout: X3_REPO from its node.env, else the first conventional path."""
    return facts.get("env", {}).get("X3_REPO") or ""


def job_script(job, repo, target_dir, min_free_gb, remote=CANONICAL_REMOTE):
    """Bash run on the worker. Prints X3JOB-META lines the controller parses."""
    q = shlex.quote
    # An explicit X3_REPO is quoted as-is; the conventional paths expand $HOME on the worker.
    repos =" ".join(([q(repo)] if repo else []) + [f'"$HOME/{r[2:]}"' for r in DEFAULT_REPOS])
    return f"""set -uo pipefail
base="$HOME/.cache/x3-cluster"; id={q(job['id'])}; sha={q(job['commit'])}
wt="$base/jobs/$id"; out="$base/out/$id"
repo=""
for r in {repos}; do [ -d "$r/.git" ] || [ -f "$r/.git" ] && {{ repo="$r"; break; }}; done
[ -n "$repo" ] || {{ echo "X3JOB-META reject=no X3 checkout on $(hostname)"; exit 97; }}
free=$(df -P --block-size=1G "$HOME" | awk 'NR==2 {{print $4}}')
echo "X3JOB-META free_gb=$free"
[ "$free" -ge {int(min_free_gb)} ] || {{ echo "X3JOB-META reject=$free GB free < {int(min_free_gb)} GB"; exit 96; }}
git -C "$repo" worktree prune
if ! git -C "$repo" cat-file -e "$sha^{{commit}}" 2>/dev/null; then
  git -C "$repo" fetch -q {q(remote)} "$sha" 2>/dev/null || git -C "$repo" fetch -q {q(remote)} 2>/dev/null
fi
git -C "$repo" cat-file -e "$sha^{{commit}}" 2>/dev/null || {{ echo "X3JOB-META reject=commit $sha not in {remote} or $repo"; exit 97; }}
mkdir -p "$base/jobs" "$out"
git -C "$repo" worktree add -q --detach "$wt" "$sha" || {{ echo "X3JOB-META reject=worktree add failed"; exit 98; }}
trap 'cd /; git -C "$repo" worktree remove --force "$wt" >/dev/null 2>&1' EXIT
export CARGO_TARGET_DIR={q(target_dir) if target_dir else '"$base/target"'}
export X3_JOB_ID="$id" X3_JOB_OUT="$out" X3_COMMIT="$sha" X3_JOB_CLASS={q(job['class'])}
command -v sccache >/dev/null && export RUSTC_WRAPPER=sccache
cd "$wt"
echo "X3JOB-META host=$(hostname)"
echo "X3JOB-META repo=$repo"
echo "X3JOB-META kernel=$(uname -r)"
echo "X3JOB-META rustc=$(rustc -V 2>/dev/null || echo none)"
echo "X3JOB-META cargo=$(cargo -V 2>/dev/null || echo none)"
echo "X3JOB-META cc=$(cc --version 2>/dev/null | head -1 || echo none)"
echo "X3JOB-META target_dir=$CARGO_TARGET_DIR"
echo "X3JOB-META start=$(date -u +%FT%TZ)"
timeout --kill-after=30 {int(job['timeout'])} nice -n {PRIORITIES[job['priority']]} bash -c {q(job['cmd'])}
rc=$?
echo "X3JOB-META end=$(date -u +%FT%TZ)"
exit $rc
"""


def parse_meta(log):
    meta = {}
    for line in log.splitlines():
        if line.startswith("X3JOB-META "):
            key, _, value = line[len("X3JOB-META "):].partition("=")
            meta[key] = value
    return meta


def result_for(code, meta):
    if code in REJECT_CODES or "reject" in meta:
        return "REJECTED"
    if code == 124 or code == 137:
        return "TIMEOUT"
    return "PASS" if code == 0 else "FAIL"


def fetch_artifacts(name, me, facts, job_id, dest):
    """Copy the worker's $X3_JOB_OUT into dest, then delete it on the worker.
    Returns [{path, sha256, bytes}]. Crash reproducers are only removed after the copy."""
    remote_out = f".cache/x3-cluster/out/{job_id}"
    dest.mkdir(parents=True, exist_ok=True)
    if name == me:
        src = Path.home() / remote_out
        if src.exists():
            shutil.copytree(src, dest, dirs_exist_ok=True)
            shutil.rmtree(src)
    else:
        host = name if name_resolves(name) else facts["_ip"]
        out = subprocess.run(ssh_cmd(host) + [f"test -d {remote_out} && tar -C {remote_out} -cf - . || true"],
                             capture_output=True, timeout=600)
        if out.returncode == 0 and out.stdout:
            with tarfile.open(fileobj=io.BytesIO(out.stdout)) as tar:
                tar.extractall(dest, filter="data")
            subprocess.run(ssh_cmd(host) + [f"rm -rf {remote_out}"], capture_output=True, timeout=60)
    files = []
    for path in sorted(p for p in dest.rglob("*") if p.is_file()):
        files.append({"path": str(path.relative_to(dest)), "sha256": sha256_file(path), "bytes": path.stat().st_size})
    return files


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def run_job(job, inv, me, evidence_root, probe_fn=probe, stream=None):
    """Route, execute and record one job. Returns the job record (also written to job.json)."""
    spec = CLASSES[job["class"]]
    need = job.get("min_free_gb") or spec["min_free_gb"]
    record = dict(job, min_free_gb=need, submitted=now(), controller=me)
    out_dir = Path(evidence_root) / job["commit"] / "jobs" / job["id"]
    out_dir.mkdir(parents=True, exist_ok=True)
    name, facts, label, rejected = choose_worker(job["class"], inv, me, probe_fn, job.get("node"), need)
    record.update(worker=name, label=label, rejected_workers=rejected)
    if name is None:
        record.update(result="REJECTED", exit_code=None,
                      detail=f"no usable {spec['role']} node" + ("" if spec["fallback"] else " and the class has no fallback"))
        (out_dir / "job.json").write_text(json.dumps(record, indent=2) + "\n")
        return record
    facts["_ip"] = inv["nodes"][name].get("ip")
    target = facts.get("env", {}).get("X3_CARGO_TARGET_DIR")
    script = job_script(job, worker_repo(facts), target, need)
    cmd = ["bash", "-s"] if name == me else ssh_cmd(name if name_resolves(name) else facts["_ip"]) + ["bash -s"]
    started = time.monotonic()
    log_path = out_dir / "log.txt"
    with open(log_path, "w") as log:
        proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        proc.stdin.write(script)
        proc.stdin.close()
        timer = threading.Timer(job["timeout"] + 120, proc.kill)  # bound SSH too, past the remote timeout
        timer.start()
        for line in proc.stdout:
            log.write(line)
            if stream:
                stream(f"[{job['id']}@{name}] {line}")
        code = proc.wait()
        proc.stdout.close()
        timer.cancel()
    text = log_path.read_text(errors="replace")
    meta = parse_meta(text)
    record.update(exit_code=code, result=result_for(code, meta), seconds=round(time.monotonic() - started, 1),
                  machine=meta.get("host", name), kernel=meta.get("kernel"), toolchain={k: meta.get(k) for k in ("rustc", "cargo", "cc")},
                  start=meta.get("start"), end=meta.get("end"), worker_repo=meta.get("repo"), target_dir=meta.get("target_dir"),
                  worker_free_gb=meta.get("free_gb"), log_sha256=sha256_file(log_path), log_tail=text[-2000:])
    if "reject" in meta:
        record["detail"] = meta["reject"]
    record["artifacts"] = fetch_artifacts(name, me, facts, job["id"], out_dir / "artifacts")
    record["finished"] = now()
    (out_dir / "job.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def new_job(cls, cmd, commit, ref, priority="NORMAL", timeout=3600, node=None, min_free_gb=None, name=None):
    if cls not in CLASSES:
        raise ValueError(f"unknown job class {cls!r}; one of {', '.join(CLASSES)}")
    if priority not in PRIORITIES:
        raise ValueError(f"unknown priority {priority!r}; one of {', '.join(PRIORITIES)}")
    if not re.fullmatch(r"[0-9a-f]{40}", commit or ""):
        raise ValueError(f"commit must be a full 40-hex SHA, got {commit!r}")
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    slug = re.sub(r"[^a-z0-9]+", "-", (name or cls).lower()).strip("-")[:32]
    return {"id": f"{stamp}-{slug}-{secrets.token_hex(3)}", "name": name or cls.lower(), "class": cls, "cmd": cmd,
            "commit": commit, "ref": ref, "priority": priority, "timeout": int(timeout), "node": node,
            "min_free_gb": min_free_gb}


def run_pipeline(spec, inv, me, commit, ref, evidence_root, probe_fn=probe, stream=None):
    """Stages in order; the jobs inside a stage run concurrently. A failed stage skips the rest."""
    stages, failed = [], False
    for stage in spec["stages"]:
        entry = {"stage": stage["name"], "jobs": []}
        if failed:
            entry["result"] = "SKIPPED"
            entry["jobs"] = [{"name": j["name"], "class": j["class"], "result": "SKIPPED"} for j in stage["jobs"]]
            stages.append(entry)
            continue
        jobs = [new_job(j["class"], j["cmd"], commit, ref, j.get("priority", "NORMAL"), j.get("timeout", 3600),
                        j.get("node"), j.get("min_free_gb"), j["name"]) for j in stage["jobs"]]
        records = [None] * len(jobs)

        def work(i):
            records[i] = run_job(jobs[i], inv, me, evidence_root, probe_fn, stream)

        threads = [threading.Thread(target=work, args=(i,)) for i in range(len(jobs))]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        required = [r for r, j in zip(records, stage["jobs"]) if not j.get("optional")]
        entry["result"] = "PASS" if all(r["result"] == "PASS" for r in required) else "FAIL"
        entry["jobs"] = [{k: r.get(k) for k in ("id", "name", "class", "worker", "label", "result", "exit_code", "seconds",
                                                 "detail")} | {"optional": bool(j.get("optional"))}
                         for r, j in zip(records, stage["jobs"])]
        failed = entry["result"] != "PASS"
        stages.append(entry)
    return {"commit": commit, "ref": ref, "controller": me, "finished": now(), "stages": stages,
            "result": "PASS" if not failed else "FAIL"}
