import json
import sys

spec_path = "chain-specs/x3-local3-raw.json"
hex_path = "/tmp/x3-rc5-runtime.hex"
out_path = "chain-specs/x3-local3-raw.patched.json"

with open(spec_path) as f:
    spec = json.load(f)
with open(hex_path) as f:
    code_hex = f.read().replace("\n", "")

spec["genesis"]["raw"]["top"]["0x3a636f6465"] = "0x" + code_hex

with open(out_path, "w") as f:
    json.dump(spec, f, indent=2)

print(f"Patched {out_path} with new runtime code blob ({len(code_hex)//2} bytes)")
