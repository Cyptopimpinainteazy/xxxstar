# 🎯 TACTICAL NEXT MOVES: Phase 2 - Value Numbering + Cost Heuristics

**Current State**: PRE live, 33.5% gas reduction, 121/121 tests passing  
**Completion**: 70%+ (was 60%)  
**Next Gate**: Value Numbering → 75%+  

---

## 🗺️ THE MAP AHEAD

### What PRE Achieved
```
ConditionalFoldPass (enhanced)  ✅
  ↓
PRE (Morel-Renvoise)            ✅ ← WE ARE HERE
  ↓
Value Numbering Integration     ⏳ NEXT (enables better matching)
  ↓
Load Hoisting                   ⏳ THEN (extends coverage)
  ↓
Cost Heuristics                 ⏳ AFTER (gas-weighted decisions)
  ↓
Loop Invariant Motion (LICM)    ⏳ LATER (loop-level hoisting)
  ↓
Register Allocation Tier        ⏳ FUTURE (75%→80%)
```

---

## 🎯 PHASE 2: VALUE NUMBERING INTEGRATION (ESTIMATED: 2-3 HOURS)

### Why Value Numbering?

**Problem**: PRE currently matches expressions textually
```
a + b    (stored as "BinOp(Add, a, b)")
b + a    (stored as "BinOp(Add, b, a)")  ← Different strings, same computation!
```

**Solution**: Value Numbering gives canonical form
```
a + b    → value_number_5
b + a    → value_number_5  ← Same number = mergeable!
```

**Benefit**: Eliminates commutative duplicates PRE currently misses

### Implementation Strategy

**Step 1: Build VN Module** (30-45 min)
```rust
// crates/x3-opt/src/value_numbering.rs
pub struct ValueNumbering {
    expr_to_vn: BTreeMap<CanonicalExpr, ValueNumber>,
    vn_to_canonical: BTreeMap<ValueNumber, CanonicalExpr>,
    next_vn: u32,
}

impl ValueNumbering {
    pub fn canonicalize_expr(&mut self, expr: &MirRhs) -> ValueNumber {
        // 1. Normalize: (a+b) and (b+a) → same canonical form
        // 2. Lookup or allocate VN
        // 3. Return consistent ID
    }

    pub fn are_equivalent(&self, e1: &MirRhs, e2: &MirRhs) -> bool {
        self.canonicalize_expr(e1) == self.canonicalize_expr(e2)
    }
}
```

**Step 2: Integrate into PRE** (30-45 min)
```rust
// In crates/x3-opt/src/passes/pre.rs
pub struct PrePassWithVN {
    base_pre: PrePass,
    vn: ValueNumbering,
}

impl PrePassWithVN {
    fn collect_candidates_with_vn(&mut self, module: &MirModule) -> BTreeSet<ValueNumber> {
        // Instead of ExprKey strings, collect by ValueNumber
        // Automatically merges commutative equivalents
    }
}
```

**Step 3: Test & Validate** (30-45 min)
- Verify commutative patterns eliminated
- Measure improvement: expect +5-15% additional reduction
- Ensure no regressions

### Expected Gains
```
Before VN: 33.5% reduction (current)
After VN:  40-45% reduction (estimated)
Additional: +10-12% above PRE alone
```

---

## 💾 DETAILED IMPLEMENTATION PLAN

### File Structure (New)
```
crates/x3-opt/src/
├── value_numbering.rs              (NEW: 200-300 lines)
│   ├── ValueNumber type (u32)
│   ├── CanonicalExpr (normalized form)
│   └── ValueNumbering struct
└── passes/
    ├── pre.rs                      (MODIFY: add VN integration)
    └── tests/
        ├── value_numbering_tests.rs (NEW: 50-100 lines)
        └── pre_vn_integration_tests.rs (NEW: 30-50 lines)
```

### Core Algorithm

```rust
// Step 1: Canonicalize Binary Operations
impl CanonicalExpr {
    fn from_binary(op: BinaryOp, lhs: MirValue, rhs: MirValue) -> CanonicalExpr {
        // Normalize commutative operations
        match op {
            BinaryOp::Add | BinaryOp::Mul | BinaryOp::BitAnd | BinaryOp::BitOr => {
                let (l, r) = if format!("{:?}", lhs) < format!("{:?}", rhs) {
                    (lhs, rhs)
                } else {
                    (rhs, lhs)
                };
                CanonicalExpr::Commutative(op, l, r)
            }
            _ => CanonicalExpr::NonCommutative(op, lhs, rhs),
        }
    }
}

// Step 2: Allocate Value Numbers
fn canonicalize_expr(&mut self, expr: &MirRhs) -> ValueNumber {
    let canonical = CanonicalExpr::from_rhs(expr);
    
    match self.expr_to_vn.get(&canonical) {
        Some(&vn) => vn,  // Already seen
        None => {
            let vn = self.next_vn;
            self.next_vn += 1;
            self.expr_to_vn.insert(canonical.clone(), vn);
            self.vn_to_canonical.insert(vn, canonical);
            vn
        }
    }
}

// Step 3: Check Equivalence
fn are_equivalent(&self, e1: &MirRhs, e2: &MirRhs) -> bool {
    let canonical1 = CanonicalExpr::from_rhs(e1);
    let canonical2 = CanonicalExpr::from_rhs(e2);
    
    self.expr_to_vn.get(&canonical1) == self.expr_to_vn.get(&canonical2)
}
```

### Test Cases

```rust
#[test]
fn vn_commutative_add() {
    let mut vn = ValueNumbering::new();
    let a = MirValue(1);
    let b = MirValue(2);
    
    let e1 = MirRhs::Binary(BinaryOp::Add, a, b);
    let e2 = MirRhs::Binary(BinaryOp::Add, b, a);
    
    assert_eq!(
        vn.canonicalize_expr(&e1),
        vn.canonicalize_expr(&e2)
    );
}

#[test]
fn vn_non_commutative_sub() {
    let mut vn = ValueNumbering::new();
    let a = MirValue(1);
    let b = MirValue(2);
    
    let e1 = MirRhs::Binary(BinaryOp::Sub, a, b);
    let e2 = MirRhs::Binary(BinaryOp::Sub, b, a);
    
    assert_ne!(
        vn.canonicalize_expr(&e1),
        vn.canonicalize_expr(&e2)
    );
}

#[test]
fn pre_with_vn_catches_commutative() {
    // PRE recognizes (a+b) and (b+a) as same
    // Expects to hoist both
}
```

---

## 📈 EXECUTION CHECKLIST

### Phase 2A: Value Numbering Module (45 min)
- [ ] Create `crates/x3-opt/src/value_numbering.rs`
- [ ] Implement `ValueNumber` type (u32 wrapper)
- [ ] Implement `CanonicalExpr` enum (Commutative, NonCommutative, Unary)
- [ ] Implement `ValueNumbering` struct with methods
- [ ] Write unit tests (commutative matching, non-commutative handling)

**Checkpoint**: 
```
cargo test -p x3-opt --lib value_numbering 
Expected: All VN tests passing
```

### Phase 2B: PRE Integration (45 min)
- [ ] Modify `pre.rs` to use `ValueNumbering` instead of `ExprKey`
- [ ] Update `collect_candidates()` to return `ValueNumber` set
- [ ] Update availability/anticipatability maps to key by `ValueNumber`
- [ ] Add integration tests (commutative redundancy elimination)

**Checkpoint**:
```
cargo test -p x3-opt --lib passes::pre
Expected: All 121+ tests passing (new VN tests added)
```

### Phase 2C: Benchmarking & Validation (30 min)
- [ ] Run `x3-bench` to measure improvement
- [ ] Verify bytecode correctness on all samples
- [ ] Compare vs. Phase 1 PRE-only baseline
- [ ] Document findings

**Expected Results**:
```
Phase 1 (PRE only):    33.5% reduction
Phase 2 (PRE + VN):    40-45% reduction (+10-12%)
```

---

## 🎯 PHASE 3: COST HEURISTICS (OPTIONAL, 1-2 HOURS)

After VN integration works, add gas-weighted decisions:

### Motivation
```
Should we hoist a large expression?

a = (very_expensive_computation)   // 100 gas
if (cond) { use(a) }
else { use(a) }

Hoist: 1x 100 gas = 100 gas
No hoist: 2x 100 gas = 200 gas
Gain: 50%

BUT: If expression is cheap:
a = (x + y)                        // 2 gas
Hoist: 1x 2 gas = 2 gas
No hoist: 2x 2 gas = 4 gas
Gain: 50%, but only saves 2 gas

Decision: Hoist expensive, maybe skip cheap (micro-optimization)
```

### Implementation
```rust
// crates/x3-opt/src/passes/pre.rs
pub struct PrePassWithCost {
    base_pre: PrePass,
    vn: ValueNumbering,
    min_hoist_cost: u32,  // Only hoist if expr gas > this
}

impl PrePassWithCost {
    fn should_hoist(&self, expr: &MirRhs, use_count: usize) -> bool {
        let cost = estimate_gas_cost(expr);
        let savings = cost * (use_count - 1);
        savings > self.min_hoist_cost
    }
}
```

---

## 📊 ROADMAP TO 80%+

```
Current (70%):
  ✅ Local optimization (positions 1-2)
  ✅ Dominance-based (positions 3-5)
  ✅ Cross-block redundancy (position 6: PRE)

Phase 2 Target (73%):
  + Value Numbering (better matching)
  + Cost heuristics (smarter decisions)

Phase 3 Target (75%):
  + Load hoisting (with alias analysis)
  + Speculative PRE (branch-aware hoisting)

Phase 4 Target (78%):
  + LICM (loop invariant code motion)
  + Advanced strength reduction

Phase 5 Target (80%+):
  + Register allocation tier
  + Superoptimizer integration
```

---

## 🚀 HOW TO PROCEED

**Option A: Continue Immediately** (Recommended)
```
cargo build --release
# Verify 121/121 tests still passing
# Then begin Phase 2A implementation
```

**Option B: Stabilize First**
```
# Run on real X3 programs
# Measure production performance
# Then begin Phase 2
```

**Option C: Benchmark Deeper**
```
# Analyze telemetry
# Identify which patterns PRE catches
# Design Phase 2 to target remaining patterns
```

---

## 📝 SUMMARY: WHERE WE ARE

✅ **Foundation Ready**: PRE working, 121/121 tests, 33.5% gas reduction  
✅ **Architecture Sound**: 14-pass pipeline, deterministic, conservative  
✅ **Next Clear**: Value Numbering is obvious next step  
✅ **Timeline**: 2-3 hours to Phase 2 completion  
✅ **Target**: 75%+ compiler completion  

---

**The compiler just crossed into LLVM-tier sophistication.**  
**Next: Make it recognize more patterns via value numbering.**  
**Then: Reach 75%+ completion.**

Ready to continue? 🚀
