#!/usr/bin/env bash
# Runs scoped checks for a trusted X3 commit; never publishes a full-CI verdict.
set -euo pipefail
sha="${1:-}"; gates="${2:-}"
[[ $# == 2 && "$sha" =~ ^[0-9a-f]{40}$ && "$gates" =~ ^[a-z0-9][a-z0-9-]*(,[a-z0-9][a-z0-9-]*)*$ ]] || { echo 'Invalid commit or gate list' >&2; exit 2; }
prefix="$HOME/.local/share/x3-ci"
mkdir -p "$prefix/jobs" "$prefix/targets"
exec 9>"$prefix/worker.lock"
flock -n 9 || { echo 'CI worker busy' >&2; exit 75; }
job="$prefix/jobs/$(date -u +%Y%m%dT%H%M%SZ)-${sha:0:12}"
mkdir -m 700 "$job"
git -C "$prefix/source" fetch --quiet origin "$sha"
[[ "$(git -C "$prefix/source" rev-parse "$sha^{commit}")" == "$sha" ]] || exit 1
git -C "$prefix/source" worktree add --quiet --detach "$job/source" "$sha"
cd "$job/source"
export RUSTUP_HOME="$prefix/rustup" RUSTUP_TOOLCHAIN=1.90.0
export PATH="$prefix/rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/bin:$prefix/bin:$PATH"
export RUSTC="$prefix/rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/bin/rustc"
export PROTOC="$prefix/bin/protoc" PROTOC_INCLUDE="$prefix/include"
export LD_LIBRARY_PATH="$prefix/lib:${LD_LIBRARY_PATH:-}"
export CARGO_HOME="$prefix/cargo" CARGO_TARGET_DIR="$prefix/targets/$sha"
export WASM_BUILD_WORKSPACE_HINT="$job/source"
export CARGO_BUILD_JOBS=4
set +e
nice -n 10 bash scripts/local-ci.sh --all --only "$gates" --jobs 1 --cargo-jobs 4 > "$job/output.log" 2>&1
result=$?
set -e
python3 - "$job" "$sha" "$gates" "$result" <<'PY'
import hashlib,json,socket,subprocess,sys,time
from pathlib import Path
job,sha,gates,code=sys.argv[1:];job=Path(job)
actual=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
dirty=subprocess.check_output(['git','status','--porcelain'],text=True).strip()
valid=actual==sha and not dirty
receipt={'commit':actual,'requested_commit':sha,'host':socket.gethostname(),'gate_selection':gates.split(','),'scope':'SCOPED; does not satisfy full x3/local-ci branch protection','exit_code':int(code),'status':'PASS' if int(code)==0 and valid else 'FAIL','source_unchanged':valid,'log_sha256':hashlib.sha256((job/'output.log').read_bytes()).hexdigest(),'recorded_at_unix':time.time()}
(job/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
if not valid:sys.exit(1)
PY
printf 'Remote evidence directory: %s\n' "$job"
exit "$result"
