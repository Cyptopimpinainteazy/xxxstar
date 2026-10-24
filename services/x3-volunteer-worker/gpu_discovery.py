"""Read-only, local GPU capability detection for X3 volunteer enrollment.

No network requests, driver installation, GPU workloads or device mutation.
All subprocesses use fixed argument arrays, never a shell.
"""
import json
import subprocess

TIMEOUT_SECONDS = 3

def _probe(argv):
    try:
        return subprocess.run(argv, capture_output=True, text=True,
                              timeout=TIMEOUT_SECONDS, check=True).stdout
    except (FileNotFoundError, subprocess.CalledProcessError,
            subprocess.TimeoutExpired, OSError):
        return ""

def discover_nvidia():
    output = _probe(["nvidia-smi", "--query-gpu=uuid,name,memory.total,compute_cap",
                     "--format=csv,noheader,nounits"])
    devices = []
    for line in output.splitlines():
        fields = [x.strip() for x in line.split(",")]
        if len(fields) != 4:
            continue
        uuid, name, memory, compute = fields
        try:
            memory_mib = int(memory)
        except ValueError:
            continue
        if not uuid.startswith("GPU-") or not name or memory_mib <= 0:
            continue
        devices.append({"vendor":"nvidia", "id":uuid, "name":name,
                        "vram_mib":memory_mib, "compute_capability":compute})
    return devices

def discover_amd():
    # No AMD driver CLI assumptions or unsafe probing: explicitly report
    # unsupported discovery until a tested ROCm/DRM adapter is implemented.
    return []

def discover():
    return {"schema_version":1, "devices":discover_nvidia() + discover_amd(),
            "execution_enabled":False, "public_enrollment_enabled":False}

if __name__ == "__main__":
    print(json.dumps(discover(), sort_keys=True))
