# RFC t5-6: Numeric Literal Coercion and Argument Type Error Policy

**Status:** DRAFT — requires review
**Scope:** `crates/x3-parser`, `crates/x3-typeck`
**Risk:** MEDIUM — affects language semantics and diagnostic consistency

---

## Problem

The current numeric literal semantics are underspecified for signed integers and function call argument mismatches.

- The parser represents `-42` as unary negation of a positive integer literal rather than a first-class signed integer literal.
- The type checker defaults integer literals to `u64` for non-negative values and uses unary negation semantics for negative values.
- Call-site mismatches between literal expressions and fixed integer parameter types can be reported via different diagnostics depending on inference path.
- Without a clear policy, it is unclear whether the compiler should perform implicit numeric coercion or always reject incompatible integer argument types.

## Current state

- `Literal::Integer(n)` is stored as an integer literal value.
- `x3-typeck` currently infers positive integer literals as `u64` and negative integer expressions via unary negation.
- There is no explicit integer literal suffix syntax for `u32`, `i64`, or similar in the parser today.
- `TypeChecker::types_compatible(...)` only treats exact primitive kind matches as compatible, except for error/never/any/typevar recovery cases.
- Existing tests now assert that function call argument mismatches should be diagnosed as `TypeErrorKind::ArgumentTypeMismatch`.

## Proposed policy

1. **Numeric literal typing**
   - A bare integer literal without a sign is typed as `u64` by default.
   - Negative values produced by `-` are handled as unary negation applied to a literal, not as a separate signed-literal kind.
   - The compiler should not perform implicit signed/unsigned coercion for integer literals unless an explicit cast or suffix syntax is added later.

2. **Argument mismatch diagnostics**
   - For function call argument type incompatibility, the only expected diagnostic should be `ArgumentTypeMismatch`.
   - Generic diagnostics such as `TypeMismatch` or `UnificationFailure` should not be used to represent direct call-site argument incompatibility.
   - This policy gives a stable signal for call-site type errors and separates parameter-compatibility failures from other type checking failures.

3. **No implicit numeric coercion**
   - The current compiler behavior should remain reject-only for mismatches between numeric literal types and parameter types.
   - If a literal is not assignable to the parameter type, the call should fail rather than be silently coerced.

## Rationale

- This policy keeps the language predictable and avoids hidden signed/unsigned coercions.
- It clarifies diagnostic expectations for developers and prevents the type checker from returning inconsistent error variants.
- It leaves room for future language evolution: explicit signed integer literal syntax or coercion rules can be added later without changing the current reject-only baseline.

## Future work

- Add parser support for explicit signed integer literals or literal suffixes once the language semantics are agreed.
- Refine tests to cover exact signed/unsigned literal behavior as the parser evolves.
- Consider a dedicated literal-typing pass for integer constants to preserve exact literal kinds beyond the current `u64` default.

## Open questions

- Should `-42` be treated as a true `i64` literal or remain unary negation of a `u64` literal until literal suffixes exist?
- Should the language ever support implicit widening or narrowing between integer literal kinds and parameter types?
- If explicit literal suffixes are added, how should they interact with current `ArgumentTypeMismatch` diagnostics?
