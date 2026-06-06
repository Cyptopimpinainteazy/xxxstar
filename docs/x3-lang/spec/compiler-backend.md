# X3 Compiler Backend Mapping (AST → Bytecode)

The compiler backend lowers AST to deterministic bytecode via several stages:

1. High-level lowering (AST -> HIR)
- Resolve names and produce an HIR with explicit constructs (closure conversion, explicit variable IDs, etc.)
- Ensure determinism by emitting stable ordering of module items and deterministic type parameter expansion.

2. Mid-level IR (MIR) - SSA-like representation
- Convert to basic blocks with explicit control-flow.
- Use deterministic numbering for temporary vars and blocks.
- Apply constant folding, simple algebraic simplifications, and dead code elimination.

3. Instruction selection
- Match MIR patterns to bytecode opcodes using a deterministic rule table.
- Use a stable mapping of calling conventions and register usage.

4. Register allocation
- Use a deterministic, simple register allocator (linear scan) with stable tie-breaking.

5. Code emission
- Emit instructions using the emitter with fixed alignment and header, produce relocation entries
- Attach debug metadata and source map for verifier

6. Verifier and optimization
- After emission, run verifier to ensure safety, then optionally run JIT hints and code layout optimizations.

Mapping table (example)
- Assignment `let a = b + c` => generate: LOAD_RAI Rtmp, [b], LOAD_RAI Rtmp2, [c], ADD_RRR Rdest, Rtmp, Rtmp2
- Function call `call f(a,b)` => CALL -> push args into reserved registers R2,R3 and CALL f
- Atomic begin/commit => ATOMIC_BEGIN .. ATOMIC_COMMIT sequences around calls

Deterministic transformations
- Sorting of map/struct fields uses alphabetical symbol order to guarantee reproducible layouts.
- Generic expansion uses deterministic instantiation order based on symbol interner IDs.

Relocations & Symbol tables
- All symbolic addresses are resolved during emission and recorded in a relocation table for runtime patching.

Debugging metadata
- Emit a `const` section that maps spans to PC offsets.
- Provide symbol tables for functions and agents.
