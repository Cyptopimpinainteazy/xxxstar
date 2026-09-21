# x3-lang current head — full gate evidence

Date: 2026-09-21 (refreshed)
Workspace: `/home/lojak/Desktop/xxxstar-main/x3-lang`

## Commands

```text
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
rustup run stable cargo test --workspace --all-targets --all-features
```

## Results

- Tests: PASS

`cargo test --workspace --all-targets --all-features` passed at the current
`master` head. The OPENSSL environment variables force openssl-sys to link the
system OpenSSL 3.6.4 rather than the Linuxbrew build whose libcrypto requires
GLIBC_2.38 (an environment-only linker issue; the code does not depend on it).

The Trading Core v1 proof suites are all present and green: typed AST/lexer,
parser/formatter, semantic hardening, risk policy, IR verifier/conformance,
atomic VM execution/rollback, receipt determinism/tamper detection, and the
property invariants (asset conservation and no unpaid-debt success).
