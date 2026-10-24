"""Local fixed-function CPU smoke test, NOT a GPU sandbox."""
import os
import subprocess
import sys

PROGRAM = "import hashlib;print(hashlib.sha256(b'x3-volunteer-fixed-benchmark-v1').hexdigest())"
EXPECTED = "ae72e6124e2e3403a26d38fbdcdaef2efdc11fa8b9f027db073c56d91158b245"

def execute(manifest, *, consent, revoked=False, timeout=5):
    if not consent or revoked:
        raise PermissionError("consent missing or revoked")
    if not isinstance(manifest, dict) or manifest != {"kind": "fixed_cpu_smoke", "version": 1}:
        raise ValueError("workload not allowlisted")
    if not isinstance(timeout, (int, float)) or isinstance(timeout, bool) or not 0 < timeout <= 5:
        raise ValueError("invalid timeout")
    result = subprocess.run(
        [sys.executable, "-I", "-S", "-c", PROGRAM],
        shell=False, capture_output=True, text=True, timeout=timeout,
        env={"PATH": os.defpath}, cwd="/", check=True,
    )
    output = result.stdout.strip()
    if output != EXPECTED:
        raise RuntimeError("unexpected benchmark output")
    return {"kind": "fixed_cpu_smoke", "sha256": output}
