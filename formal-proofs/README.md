# Formal Proofs

This directory holds the formal-method specifications that pair with the
executable test suite under `pallets/` and the proof receipts under
`proof/receipts/claims/`. The companion runner is
`proof-forge::runners::check_formal_proofs` and the CI driver is
`.github/workflows/formal-verification.yml`.

## Layout

```
formal-proofs/
├── tla/        # TLA+ specifications (model checked with TLC)
│   └── asset_kernel/
│       ├── AssetKernelSupply.tla   ← supply_conservation invariant (P0)
│       └── AssetKernelSupply.cfg   ← TLC config (Accounts, Bridges, MaxOp)
├── coq/        # Coq proofs (machine-checked theorems)
│   ├── SupplyInvariant.v           ← supply conservation state-transition proof
│   └── README.md
├── k/          # K Framework executable semantics
│   ├── x3vm-spec.k                ← baseline VM deterministic reduction rules
│   └── README.md
└── dafny/      # (optional) Dafny specs — advisory only
```

## How to run TLC locally

```bash
# Requires Java 11+ and a TLA+ tools jar on PATH (tla2tools.jar).
java -cp tla2tools.jar tlc2.TLC \
  -config formal-proofs/tla/asset_kernel/AssetKernelSupply.cfg \
       formal-proofs/tla/asset_kernel/AssetKernelSupply.tla
```

Expected: `Model checking completed. No error has been found.` for the
default configuration (`Accounts = {a1, a2, a3}`, `Bridges = {b1, b2}`,
`MaxOp = 6`). Increase `MaxOp` for deeper exploration; complexity grows
roughly with `|Operations|^MaxOp`.

## How proof-forge consumes this directory

`x3-proof formal --strict --report` executes backend runners across TLA+, Coq,
and K specs. In strict mode it fails closed when proofs fail, or when required
tooling/specs are missing.
