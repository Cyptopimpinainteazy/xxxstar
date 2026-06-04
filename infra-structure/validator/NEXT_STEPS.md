# Next Steps for Validator Infrastructure

The following items remain to be completed:

1. **Modify CLI verifier selection** – update `cli.py` to select verifiers based on `SignatureAlgorithm` and add ED25519 verifier paths.
2. **Enforce public‑key sizes** – add validation in `svm_validator.py`, `substrate/__init__.py`, and `chain_registry.py` to ensure correct key sizes.
3. **Require real Keccak provider** – adjust `keccak_gpu.py` and `pyproject.toml` to depend on a proper Keccak library and fail if missing.
4. **Update tests** – modify `tests/cross_chain_gpu_validator/test_gpu_parity.py` to use true Keccak vectors and add regression test for SHA3‑256 rejection.
5. **Run the full test suite** to verify that all changes compile and pass.

These steps can be tackled sequentially. Let me know which one you would like me to address next.