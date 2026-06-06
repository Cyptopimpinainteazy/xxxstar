# CUDA Kernels

This folder contains CUDA kernels used by the cross-chain validator.

- `secp256k1_batch.cu`: batch signature verification
- `keccak256_batch.cu`: batch keccak256 hashing

Build with:

```bash
./build.sh
```

## Notes

The kernels ship with placeholder implementations and MUST be replaced
with real secp256k1 and keccak256 CUDA kernels before production use.

The secp256k1 kernel includes `secp256k1.cuh` from
`third_party/secp256k1-cuda-ecc` (MIT License).
