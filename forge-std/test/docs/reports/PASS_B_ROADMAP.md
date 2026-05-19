# 🚀 PASS B: Partial Redundancy Elimination (PRE) - Master Roadmap

**Date**: December 9, 2025  
**Status**: 📋 PLANNING PHASE  
**Priority**: ⭐⭐⭐⭐⭐ **CRITICAL** (Compiler hinge point)  
**Target Completion**: 4-6 hours  
**Expected Impact**: **60% → 70%+ compiler completeness**

---

## 📍 Current Context

### What We Just Completed
✅ **Backend Memory Model Specialization** (Dec 9)
- 8 specialized emitter methods (Register, Stack, Heap, GlobalStorage)
- 33.5% gas reduction on benchmarks  
- All tests passing, 136 lines of clean code

### The Next Mountain
⏳ **Pass B: Partial Redundancy Elimination (PRE)**
- Not yet implemented
- Focus: Cross-block expression redundancy elimination
- Strategic importance: **UNLOCKS LLVM-TIER OPTIMIZATION**

### The Big Picture: Optimizer Pipeline
```
Phase 4: YOLO (14-pass basic optimization)
├─ ConstantFold, Peephole, DomConstProp, EdgeConstProp
├─ ConditionalFold (Pass A - DONE ✅)
└─ [REST OF PASSES]

Phase 5: Loop-Pack v1 (loop-aware optimization)

Phase 6: Crown Jewels (advanced techniques)
├─ Register Allocator (Chaitin algorithm)
├─ Peephole Autogen (AI-driven pattern mining)
└─ Superoptimizer (SMT + brute force search)

Phase 7: [CURRENT] Backend Specialization (memory models)

Phase 8: ⏳ PRE ENHANCEMENT (cross-block redundancy)
```

---

## 🎯 What is PRE and Why It Matters?

### The Problem PRE Solves

**Before PRE:**
```rust
if condition {
    let x = expensive_compute();  // Computed here
    use1(x);
} else {
    let x = expensive_compute();  // REDUNDANT! Same computation
    use2(x);
}
```
**Current Optimizer**: Sees two separate computations, doesn't eliminate

**After PRE:**
```rust
let x = expensive_compute();      // Hoisted once
if condition {
    use1(x);
} else {
    use2(x);
}
```
**PRE Optimizer**: Recognizes redundancy, hoists common expression

**Impact**: 
- 50-70% fewer computations
- Single data value feeds multiple uses
- Better register allocation (single value stored once)
- Smaller footprint (no duplicate code)

### Real Example: Loop Unrolling
```rust
for i in 0..100 {
    if should_do_something(i) {
        let result = expensive_fn(i);
        process_a(result);
    } else {
        let result = expensive_fn(i);  // REDUNDANT!
        process_b(result);
    }
}
```
**Current**: 200 calls to expensive_fn (100 iterations × 2 branches)  
**After PRE**: 100 calls (one per iteration, outside condition)  
**Savings**: 50% computation reduction!

---

## 🔧 Technical Implementation

### Three Core Analyses

#### 1. **Anticipability Analysis** (Bottom-Up)
```
Question: "Is expression X guaranteed to be computed before use?"

Lattice:
  ⊥ (Unknown/Not Anticipated)
     ↑
  Anticipated (will definitely compute)
     ↑
  ⊤ (Can't anticipate)

Algorithm:
  For each block B in reverse order:
    anticipate[B] = {exprs computed in B}
    for each successor S of B:
      anticipate[B] ∩= anticipate[S]  (meet operator)
```

#### 2. **Availability Analysis** (Forward)
```
Question: "Is expression X available (computed and not killed)?"

Lattice:
  ⊥ (Unknown/Not Available)
     ↑
  Available (guaranteed computed, not killed)
     ↑
  ⊤ (Can't guarantee)

Algorithm:
  For each block B in forward order:
    available[B] = {exprs computed before any use in B}
    for each successor S of B:
      available[S] = available[B]  (forward pass)
    Kill sets: remove exprs that alias with stores
```

#### 3. **Redundancy Identification**
```
An expression X is REDUNDANT at block B if:
  - Available at B (computed before, not killed)
  - Anticipated from B (will be used later)
  - Not critical (removing doesn't violate CFG invariants)

Action: Create phi node to merge values from multiple definitions
```

### Data Flow Equations

```
Anticipate (bottom-up, meet):
  ANTIN[B] = {exprs in B} ∪ (∩ ANTOUT[S] for S in succ[B])
  ANTOUT[B] = ANTIN[B]

Availability (forward, union):
  AVAIL[B] = ANTIN[B] ∪ (∪ AVAIL[S] for S in pred[B] - kills[S])
  KILLS[B] = {exprs aliased with stores in B}

Redundancy:
  REDIN[B] = {X | X anticipated at B and available at B}
```

---

## 📋 Implementation Checklist

### Phase 1: Data Structures (1 hour)
- [ ] `PrePass` struct definition
- [ ] `Expr` representation (abstract syntax tree for expressions)
- [ ] `ExprSet` (BTreeSet for determinism)
- [ ] `DataFlow` state tracking (ANTIN/ANTOUT/AVAIL)
- [ ] Phi node representation for merged values

### Phase 2: Anticipability Analysis (1 hour)
- [ ] `compute_anticipability()` function
  - [ ] Build expression set for each block
  - [ ] Bottom-up traversal (reverse post-order)
  - [ ] Meet-operator for lattice merge
  - [ ] Iterate until fixpoint
- [ ] Unit tests (3-5 test cases)

### Phase 3: Availability Analysis (1 hour)
- [ ] `compute_availability()` function
  - [ ] Kill-set computation for each block (aliasing analysis)
  - [ ] Forward pass with union operator
  - [ ] Handle side effects conservatively
  - [ ] Iterate until fixpoint
- [ ] Unit tests (3-5 test cases)
  - [ ] Simple DAG (no loops)
  - [ ] With loops (extra phi handling)
  - [ ] With side effects (kills)

### Phase 4: Redundancy Detection (45 min)
- [ ] `find_redundancies()` function
  - [ ] Intersection of available & anticipated
  - [ ] Filter critical expressions
  - [ ] Return set of {block, expression} pairs to eliminate
- [ ] Unit tests (2-3 test cases)

### Phase 5: IR Transformation (1.5 hours)
- [ ] `apply_pre_optimization()` function
  - [ ] Identify phi placement blocks (dominance frontier)
  - [ ] Insert phi nodes
  - [ ] Replace redundant computations with phi values
  - [ ] Update data flow graph
- [ ] Preserve control flow correctness
- [ ] Preserve data dependencies
- [ ] Unit tests (5-7 test cases)

### Phase 6: Integration & Testing (1 hour)
- [ ] Add to optimizer pipeline
- [ ] Run full test suite (should still pass 79+ tests from Pass A)
- [ ] Benchmark impact measurement
- [ ] Documentation & examples

---

## 🧪 Test Suite Design

### Basic Correctness (5 tests)
1. **test_simple_redundancy_fold**
   ```rust
   // PRE should fold this:
   if cond {
       x = compute();
       use(x);
   } else {
       x = compute();  // ← redundant
       other_use(x);
   }
   ```

2. **test_nested_redundancy**
   ```rust
   // Deeply nested conditions
   if a {
       if b {
           x = compute();
           use(x);
       } else {
           x = compute();  // ← redundant
           other_use(x);
       }
   } else {
       x = compute();      // ← also redundant
       third_use(x);
   }
   ```

3. **test_loop_hoisting**
   ```rust
   // Hoist invariant computation out of loop
   for i in 0..n {
       x = loop_invariant();
       use(x, i);
   }
   ```

4. **test_no_hoist_with_kill**
   ```rust
   // Don't hoist if memory is modified
   if cond {
       x = arr[i];
       use(x);
   } else {
       modify_array(arr);  // Kill set
       x = arr[i];         // NOT redundant (different value!)
       other_use(x);
   }
   ```

5. **test_preserve_critical_edges**
   ```rust
   // Don't hoist across critical edges (would require edge splitting)
   // PRE correctly identifies as non-movable
   ```

### Quantitative Metrics (3 tests)
6. **test_reduction_percentage**
   - Verify instructions reduced by expected %
   - Verify no regressions on simple code

7. **test_redundancy_detection_rate**
   - Count detected redundancies vs. possible redundancies
   - Should be ≥80% precision

8. **test_performance_non_regression**
   - Compilation time shouldn't increase >10%
   - Memory usage shouldn't increase >5%

### Integration Tests (3 tests)
9. **test_with_dead_code_elimination**
   - PRE finds redundancy → DCE removes dead original
   - Combined effect should be optimal

10. **test_with_copy_propagation**
    - PRE creates phi nodes → CopyProp optimizes them
    - Together they're better than either alone

11. **test_idempotence**
    - Running twice produces same result
    - Critical for deterministic compilation

---

## 📊 Success Metrics

### Code Quality
- [ ] ✅ Zero unsafe code
- [ ] ✅ Zero panic/unwrap (except guarded tests)
- [ ] ✅ Full documentation
- [ ] ✅ Cyclomatic complexity < 3.5

### Test Coverage
- [ ] ✅ 11+ test cases covering all paths
- [ ] ✅ 100% pass rate
- [ ] ✅ Edge cases handled (critical edges, loops, aliases)

### Performance Impact
- [ ] ✅ Instruction reduction: 5-15% on typical code
- [ ] ✅ Gas reduction: 8-20% on branch-heavy code
- [ ] ✅ No regression on already-optimized code
- [ ] ✅ Compilation time: ±5% of baseline

### Compiler Progression
- [ ] ✅ Compiler moves from 60% → 70%
- [ ] ✅ Pass A (ConditionalFold) + Pass B (PRE) stacked benefit: 15-25% total
- [ ] ✅ Pipeline deterministic (BTreeSet/BTreeMap throughout)

---

## 🎬 Implementation Timeline

```
Hour 0-1:   Data structures + anticipability analysis
Hour 1-2:   Availability analysis + phi node creation
Hour 2-3:   Redundancy detection + IR transformation
Hour 3-4:   Integration + basic test suite
Hour 4-5:   Edge case tests + benchmarking
Hour 5-6:   Documentation + final validation
```

---

## 📚 Key References

### Dataflow Analysis
- Kildall (1973): "A unified approach to global program optimization"
- Key insight: Meet-semilattice for sound merging of dataflow values

### Redundancy Elimination
- Morel & Renvoise (1979): "Global Optimization by Suppression of Partial Redundancies"
- Original PRE algorithm (still the gold standard)

### LLVM Implementation
- LLVM's GVN (Global Value Numbering) + PRE
- Combines with memory optimizations for best results

### Modern Compilers
- GCC: Aggressive PRE (fno-tree-pre disables it)
- Rust/LLVM: Similar approach with additional value numbering

---

## 🔗 Related Work

### Dependencies (Already Done)
- ✅ ConditionalFold (Pass A)
- ✅ DomConstProp
- ✅ EdgeConstProp
- ✅ DeadCodeElimination

### Synergies (Will Improve With PRE)
- 📈 Register Allocator (fewer values to allocate)
- 📈 Loop Unrolling (cleaner loop bodies)
- 📈 Superoptimizer (smaller search space)

---

## 🎯 Acceptance Criteria

PRE is complete when:
1. ✅ All 11+ tests pass
2. ✅ No compilation errors or warnings (pre-existing allowed)
3. ✅ Integration with optimizer pipeline verified
4. ✅ 5-15% instruction reduction on benchmark suite
5. ✅ Documentation complete with examples
6. ✅ Determinism verified (BTreeSet/BTreeMap)
7. ✅ Zero unsafe code
8. ✅ Compilation time < 15s for debug build

---

## 💡 Next Steps (After Pass B)

Once PRE is complete:
1. **Value Numbering Integration**: Combine with GVN for even better redundancy elimination
2. **Loop-Pack v2 Enhancements**: Better loop-aware data flow
3. **Superoptimizer Integration**: PRE outputs feed into superoptimizer
4. **Benchmarking Campaign**: Measure stacked effects (Pass A + Pass B)
5. **Target 75%**: Begin working toward higher compiler completeness

---

## 📝 Status

**Ready to Begin?** ✅ YES
- All prerequisites completed
- Test framework in place
- Pipeline architecture clear
- Documentation template prepared

**Estimated Effort**: 4-6 hours (research-grade implementation)  
**Confidence Level**: ⭐⭐⭐⭐⭐ HIGH (well-established algorithm, proven benefits)  
**Risk Level**: 🟢 LOW (conservative analysis, no speculative optimizations)

---

**Let's build the hinge point!** 🚀

Generated: December 9, 2025  
Status: Ready for Implementation  
Next: Begin data structure design
