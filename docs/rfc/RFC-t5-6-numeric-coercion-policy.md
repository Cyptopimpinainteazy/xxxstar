# RFC t5-6: Numeric Literal Coercion and Argument Type Error Policy

**Status:** ACCEPTED for X3Lang 1.0 baseline
**Scope:** canonical Rust compiler path under `x3-lang/compiler`; root `crates/x3-typeck` remains a compatibility/integration implementation and must not define divergent language semantics
**Risk:** MEDIUM — affects language semantics and diagnostic consistency

---

## Decision

X3Lang 1.0 uses a conservative, deterministic integer policy:

1. A bare non-negative integer literal is `u64`.
2. A negative integer such as `-42` remains represented in the AST as unary negation applied to a positive integer literal; for numeric compatibility, the resulting expression type is `i64` for the current bare-literal baseline.
3. Integer widths and signedness must match exactly at direct function-call boundaries.
4. There is no implicit widening, narrowing, or signed/unsigned integer coercion.
5. Direct function argument incompatibility reports the stable compiler diagnostic `X3E0202` (`ArgumentTypeMismatch`).
6. Explicit literal suffix/cast syntax is not part of the current source-language contract. AST suffix variants are reserved for future syntax/tooling and must not be used to imply currently unsupported source syntax.

## Rationale

- Exact compatibility keeps compilation deterministic and prevents hidden value-changing conversions.
- Bare literals have one predictable default (`u64`).
- Unary negation preserves the parser model instead of inventing a separate signed-literal token kind.
- Treating the resulting bare negative expression as `i64` gives X3Lang a usable signed baseline while retaining unary-negation AST semantics.
- A dedicated call-site diagnostic gives tooling a stable machine-readable signal independent of diagnostic wording.

## Required examples

These are the X3Lang 1.0 baseline behaviors:

```x3
fn takes_u64(x: u64) { }
fn main() { takes_u64(1); }
```

Accepted: bare `1` is `u64`.

```x3
fn takes_i64(x: i64) { }
fn main() { takes_i64(-1); }
```

Accepted: `-1` is unary negation and the resulting bare negative integer expression is `i64`.

```x3
fn takes_i64(x: i64) { }
fn main() { takes_i64(1); }
```

Rejected with `X3E0202`: no unsigned-to-signed coercion.

```x3
fn takes_u64(x: u64) { }
fn main() { takes_u64(-1); }
```

Rejected with `X3E0202`: no signed-to-unsigned coercion.

```x3
fn takes_u32(x: u32) { }
fn main() { takes_u32(1); }
```

Rejected with `X3E0202`: bare `1` is `u64`; implicit narrowing to `u32` is forbidden.

## Implementation authority

The accepted policy is implemented and regression-tested in the canonical Rust language workspace:

- `x3-lang/compiler/src/numeric.rs`
- `x3-lang/compiler/tests/test_numeric_policy.rs`
- stable codes in `x3-lang/compiler/src/diagnostic.rs`

The root workspace `crates/x3-parser` / `crates/x3-typeck` may temporarily retain older behavior while compatibility migration is in progress, but those crates do not supersede this policy. Differential tests should be used during migration where practical.

## Future work

Future RFCs may add:

- explicit integer literal suffix syntax;
- explicit cast syntax;
- checked widening conversions;
- literal-range-aware inference;
- richer first-class numeric types.

Any such change is a language-semantic change and requires explicit specification and conformance coverage. It must not silently alter the X3Lang 1.0 baseline.
