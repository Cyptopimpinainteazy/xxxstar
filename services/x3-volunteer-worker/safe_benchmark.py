"""Local fixed-function CPU smoke test, NOT a GPU sandbox."""
import os
import subprocess
import sys

PROGRAM = "import hashlib;print(hashlib.sha256(b'x3-volunteer-fixed-benchmark-v1').hexdigest())"
EXPECTED = "ae72e6124e2e3403a26d38fbdcdaef2efdc11fa8b9f027db073c56d91158b245"

def authorize_controls(*, consent, revoked=False, stop_requested=False):
    # Truthy strings or integers are not an owner's explicit opt-in. Unknown
    # revocation/stop values must also fail closed, even when falsey in Python.
    if consent is not True or revoked is not False or stop_requested is not False:
        raise PermissionError("worker not accepting jobs: invalid or denied consent controls")


def validate_manifest(manifest):
    # Python considers True == 1 and 1.0 == 1; wire protocol versions do not.
    if (type(manifest) is not dict or set(manifest) != {"kind", "version"}
            or manifest["kind"] != "fixed_cpu_smoke"
            or type(manifest["version"]) is not int or manifest["version"] != 1):
        raise ValueError("workload not allowlisted")


def execute(manifest, *, consent, revoked=False, timeout=5):
    authorize_controls(consent=consent, revoked=revoked)
    validate_manifest(manifest)
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
